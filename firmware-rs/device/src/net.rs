//! The box on Wi-Fi (ADR-0012, docs/protocol.md "Local network link"): joining the network the user set
//! over USB, DHCP, and the TCP server computers on the local network connect to. Who may link is
//! decided by firmware-core (`lan.rs`); this only moves bytes. The main loop owns the firmware, so
//! everything crosses over in static channels: [`EVENTS`] towards the main loop, a pipe per
//! connection and [`CREDENTIALS`] away from it.

use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU32, Ordering};

use embassy_executor::Spawner;
use embassy_futures::select::{Either3, select, select3};
use embassy_net::tcp::TcpSocket;
use embassy_net::{Runner, Stack, StackResources};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;
use embassy_sync::pipe::Pipe;
use embassy_sync::signal::Signal;
use embassy_time::{Duration, Timer};
use esp_hal::rng::Rng;
use esp_radio::wifi::sta::StationConfig;
use esp_radio::wifi::{AuthenticationMethodConfig, Config, ControllerConfig, Interface, WifiController};
use static_cell::StaticCell;

/// The port computers connect to (docs/protocol.md).
const PORT: u16 = 7340;
/// Connections served at once: the linked computer, and one more so a takeover can get through.
const SLOTS: usize = 2;
/// Sockets beyond the servers: DHCP, DNS, SNTP and the Relay.
const EXTRA_SOCKETS: usize = 4;
const SOCKET_RX_BYTES: usize = 2048;
const SOCKET_TX_BYTES: usize = 4096;
/// Output waiting for a connection; a screenshot is the biggest burst.
const PIPE_BYTES: usize = 4096;

/// What the network side tells the main loop.
pub enum NetEvent {
    /// A diagnostic line for the computer: `WIFI CONNECTED`, `WIFI ADDRESS <ip>`, `WIFI FAILED <why>`.
    Line(String),
    Opened(u32, [u8; 32]),
    Received(u32, Vec<u8>),
    Closed(u32),
}

pub static EVENTS: Channel<CriticalSectionRawMutex, NetEvent, 16> = Channel::new();
/// The computer on the cable asked for the networks the box can hear.
pub static SCAN: Signal<CriticalSectionRawMutex, ()> = Signal::new();
/// The network to join, or None to leave Wi-Fi; signalled at boot and whenever the user changes it.
pub static CREDENTIALS: Signal<CriticalSectionRawMutex, Option<(String, String)>> = Signal::new();

static OUT: [Pipe<CriticalSectionRawMutex, PIPE_BYTES>; SLOTS] = [Pipe::new(), Pipe::new()];
static CLOSE: [Signal<CriticalSectionRawMutex, ()>; SLOTS] = [Signal::new(), Signal::new()];
/// The connection number each slot serves now; 0 when it serves none.
static SLOT_CONN: [AtomicU32; SLOTS] = [AtomicU32::new(0), AtomicU32::new(0)];
static NEXT_CONN: AtomicU32 = AtomicU32::new(1);

fn slot_of(conn: u32) -> Option<usize> {
    (conn != 0).then(|| SLOT_CONN.iter().position(|slot| slot.load(Ordering::Relaxed) == conn)).flatten()
}

/// Queues bytes for a connection; dropped when it can't take more, like the serial port's output.
pub fn send(conn: u32, bytes: &[u8]) {
    if let Some(slot) = slot_of(conn) {
        let _ = OUT[slot].try_write(bytes);
    }
}

/// Closes every connection, as before a restart.
pub fn close_all() {
    for signal in &CLOSE {
        signal.signal(());
    }
}

pub fn close(conn: u32) {
    if let Some(slot) = slot_of(conn) {
        CLOSE[slot].signal(());
    }
}

/// Starts Wi-Fi and the server. Nothing joins a network until [`CREDENTIALS`] names one. Returns the
/// network stack for the Relay task, or None if Wi-Fi didn't start.
pub fn start(spawner: &Spawner, wifi: esp_hal::peripherals::WIFI<'static>) -> Option<Stack<'static>> {
    let Ok(mut controller) = WifiController::new(wifi, ControllerConfig::default()) else {
        let _ = EVENTS.try_send(NetEvent::Line(String::from("WIFI FAILED init")));
        return None;
    };
    // esp-radio's default ceiling is 5 dBm, too weak for some access points to hear the handshake;
    // 80 (quarter dBm) is ESP-IDF's default of 20 dBm.
    let _ = controller.set_max_tx_power(80);
    let rng = Rng::new();
    let seed = u64::from(rng.random()) << 32 | u64::from(rng.random());
    static RESOURCES: StaticCell<StackResources<{ SLOTS + EXTRA_SOCKETS }>> = StaticCell::new();
    let (stack, runner) = embassy_net::new(
        Interface::station(),
        embassy_net::Config::dhcpv4(Default::default()),
        RESOURCES.init(StackResources::new()),
        seed,
    );
    spawner.spawn(connection(controller).expect("Wi-Fi task"));
    spawner.spawn(net_task(runner).expect("network task"));
    spawner.spawn(address_task(stack).expect("address task"));
    for slot in 0..SLOTS {
        spawner.spawn(server(stack, slot).expect("server task"));
    }
    Some(stack)
}

fn line(text: String) {
    let _ = EVENTS.try_send(NetEvent::Line(text));
}

#[embassy_executor::task]
async fn connection(mut controller: WifiController<'static>) {
    let mut wanted: Option<(String, String)> = None;
    loop {
        // Wait for a network to join, or a change while not connected.
        if wanted.is_none() {
            match select(CREDENTIALS.wait(), SCAN.wait()).await {
                embassy_futures::select::Either::First(next) => wanted = next,
                embassy_futures::select::Either::Second(()) => report_scan(&mut controller).await,
            }
            continue;
        }
        let Some((ssid, password)) = wanted.clone() else { continue };
        let auth = if password.is_empty() {
            AuthenticationMethodConfig::Open
        } else {
            match password.as_str().try_into() {
                Ok(password) => AuthenticationMethodConfig::Wpa2Personal(password),
                Err(_) => {
                    line(String::from("WIFI FAILED password"));
                    wanted = CREDENTIALS.wait().await;
                    continue;
                }
            }
        };
        let Ok(ssid_value) = ssid.as_str().try_into() else {
            line(String::from("WIFI FAILED ssid"));
            wanted = CREDENTIALS.wait().await;
            continue;
        };
        let config = Config::Station(StationConfig::default().with_ssid(ssid_value).with_authentication(auth));
        if controller.set_config(&config).is_err() {
            line(String::from("WIFI FAILED config"));
            wanted = CREDENTIALS.wait().await;
            continue;
        }
        line(alloc::format!("WIFI CONNECTING {ssid}"));
        match select(controller.connect_async(), CREDENTIALS.wait()).await {
            embassy_futures::select::Either::First(Ok(_)) => {
                line(String::from("WIFI CONNECTED"));
                // Scans can run while connected; the station stays on its network.
                let next = loop {
                    match select3(controller.wait_for_disconnect_async(), CREDENTIALS.wait(), SCAN.wait()).await {
                        Either3::First(_) => break None,
                        Either3::Second(next) => break Some(next),
                        Either3::Third(()) => report_scan(&mut controller).await,
                    }
                };
                match next {
                    None => line(String::from("WIFI DISCONNECTED")),
                    Some(next) => {
                        let _ = controller.disconnect_async().await;
                        wanted = next;
                        if wanted.is_none() {
                            line(String::from("WIFI DISCONNECTED"));
                        }
                        continue;
                    }
                }
            }
            embassy_futures::select::Either::First(Err(error)) => {
                line(alloc::format!("WIFI FAILED {error:?}"));
                report_scan(&mut controller).await;
            }
            embassy_futures::select::Either::Second(next) => {
                wanted = next;
                continue;
            }
        }
        // Try again in a while, sooner if the user sets another network.
        if let embassy_futures::select::Either::Second(next) = select(Timer::after(Duration::from_secs(10)), CREDENTIALS.wait()).await {
            wanted = next;
        }
    }
}

/// `WIFI SEEN <rssi> <channel> <ssid>` for what the box can hear, strongest first: when it can't join,
/// this tells a 5 GHz-only network or a mistyped name from a box that hears nothing.
async fn report_scan(controller: &mut WifiController<'static>) {
    let config = esp_radio::wifi::scan::ScanConfig::default().with_max(12);
    match controller.scan_async(&config).await {
        Ok(mut found) => {
            found.sort_by_key(|ap| core::cmp::Reverse(ap.signal_strength));
            line(alloc::format!("WIFI SEEN {}", found.len()));
            for ap in found {
                line(alloc::format!("WIFI SEEN {} {} {}", ap.signal_strength, ap.channel, ap.ssid.as_str()));
            }
        }
        Err(error) => line(alloc::format!("WIFI SCAN FAILED {error:?}")),
    }
}

#[embassy_executor::task]
async fn net_task(mut runner: Runner<'static, Interface>) {
    runner.run().await
}

/// Reports the address each time the network comes up, so the computer on the cable can find the box.
#[embassy_executor::task]
async fn address_task(stack: Stack<'static>) {
    loop {
        stack.wait_config_up().await;
        if let Some(config) = stack.config_v4() {
            line(alloc::format!("WIFI ADDRESS {}", config.address.address()));
        }
        stack.wait_config_down().await;
    }
}

/// One connection at a time per slot: accept, hand it to firmware-core by number, shuttle bytes.
#[embassy_executor::task(pool_size = SLOTS)]
async fn server(stack: Stack<'static>, slot: usize) {
    // On the heap, which is PSRAM first: internal RAM is for the Wi-Fi driver and the stack.
    let mut rx = alloc::vec![0u8; SOCKET_RX_BYTES];
    let mut tx = alloc::vec![0u8; SOCKET_TX_BYTES];
    let mut chunk = [0u8; 256];
    loop {
        let mut socket = TcpSocket::new(stack, &mut rx, &mut tx);
        socket.set_timeout(Some(Duration::from_secs(60)));
        if socket.accept(PORT).await.is_err() {
            Timer::after(Duration::from_millis(100)).await;
            continue;
        }
        let conn = NEXT_CONN.fetch_add(1, Ordering::Relaxed);
        OUT[slot].clear();
        CLOSE[slot].reset();
        SLOT_CONN[slot].store(conn, Ordering::Relaxed);
        // With the radio on, the RNG draws on real noise.
        let mut nonce = [0u8; 32];
        Rng::new().read(&mut nonce);
        EVENTS.send(NetEvent::Opened(conn, nonce)).await;
        let mut out = [0u8; 256];
        loop {
            match select3(socket.read(&mut chunk), OUT[slot].read(&mut out), CLOSE[slot].wait()).await {
                Either3::First(Ok(count @ 1..)) => EVENTS.send(NetEvent::Received(conn, chunk[..count].to_vec())).await,
                Either3::First(_) => break,
                Either3::Second(count) => {
                    if embedded_io_async::Write::write_all(&mut socket, &out[..count]).await.is_err() {
                        break;
                    }
                }
                Either3::Third(()) => {
                    // Let what was queued before the close (a refusal, say) go out first.
                    while let Ok(count) = OUT[slot].try_read(&mut out) {
                        if embedded_io_async::Write::write_all(&mut socket, &out[..count]).await.is_err() {
                            break;
                        }
                    }
                    let _ = embedded_io_async::Write::flush(&mut socket).await;
                    break;
                }
            }
        }
        SLOT_CONN[slot].store(0, Ordering::Relaxed);
        socket.close();
        socket.abort();
        EVENTS.send(NetEvent::Closed(conn)).await;
    }
}
