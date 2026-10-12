//! Confirming an image installed over the air (ADR-0013). The ESP-IDF bootloader, built with app
//! rollback, marks a newly selected image PENDING_VERIFY on its first boot; unless the image marks
//! itself VALID before the next boot, the bootloader aborts it and goes back to the other slot.
//!
//! The otadata partition holds two sectors, each starting with a 32-byte entry: the sequence number
//! (which slot is selected), a label, the state, and a CRC of the sequence number. The entry with
//! the highest valid sequence is the one the bootloader followed. This reads and writes them
//! directly: esp-bootloader-esp-idf's helper parses the whole partition table and panics on our
//! custom data subtypes (voices, link), which a box showed on 2026-10-12.

use crate::storage::{Flash, FlashError, Region, SECTOR_BYTES};
use crate::voice_pack::crc32;

const ENTRY_BYTES: usize = 32;
const STATE_NEW: u32 = 0;
const STATE_PENDING_VERIFY: u32 = 1;
const STATE_VALID: u32 = 2;
const UNSET: u32 = 0xFFFF_FFFF;

/// What confirming did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Confirmed {
    /// The running image was waiting to be confirmed, and now is.
    Now,
    /// Nothing waited: otadata is blank (a USB flash) or the image was confirmed before.
    Nothing,
}

fn word(entry: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([entry[at], entry[at + 1], entry[at + 2], entry[at + 3]])
}

/// The CRC ESP-IDF keeps over an entry's sequence number: crc32 seeded with all ones.
pub fn sequence_crc(sequence: u32) -> u32 {
    crc32(UNSET, &sequence.to_le_bytes())
}

/// Marks the selected image VALID if the bootloader left it NEW or PENDING_VERIFY.
pub fn confirm(flash: &mut dyn Flash, otadata: Region) -> Result<Confirmed, FlashError> {
    let mut current: Option<(u32, u32)> = None;
    for sector in 0..2 {
        let mut entry = [0u8; ENTRY_BYTES];
        flash.read(otadata.offset + sector * SECTOR_BYTES, &mut entry)?;
        let sequence = word(&entry, 0);
        if sequence == UNSET || word(&entry, 28) != sequence_crc(sequence) {
            continue;
        }
        if current.is_none_or(|(_, best)| sequence > best) {
            current = Some((sector, sequence));
        }
    }
    let Some((sector, _)) = current else { return Ok(Confirmed::Nothing) };
    let offset = otadata.offset + sector * SECTOR_BYTES;
    let mut entry = [0u8; ENTRY_BYTES];
    flash.read(offset, &mut entry)?;
    if !matches!(word(&entry, 24), STATE_NEW | STATE_PENDING_VERIFY) {
        return Ok(Confirmed::Nothing);
    }
    // NOR flash can only clear bits, and VALID sets one, so the entry's sector is rewritten.
    let mut page = [0u8; ENTRY_BYTES];
    page.copy_from_slice(&entry);
    page[24..28].copy_from_slice(&STATE_VALID.to_le_bytes());
    flash.erase(offset, offset + SECTOR_BYTES)?;
    flash.write(offset, &page)?;
    Ok(Confirmed::Now)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::tests::MemoryFlash;

    const OTADATA: Region = Region { offset: 0x2000, size: 0x2000 };

    fn entry(sequence: u32, state: u32) -> [u8; ENTRY_BYTES] {
        let mut entry = [0xFFu8; ENTRY_BYTES];
        entry[0..4].copy_from_slice(&sequence.to_le_bytes());
        entry[24..28].copy_from_slice(&state.to_le_bytes());
        entry[28..32].copy_from_slice(&sequence_crc(sequence).to_le_bytes());
        entry
    }

    fn put(flash: &mut MemoryFlash, sector: u32, bytes: &[u8]) {
        let at = (OTADATA.offset + sector * SECTOR_BYTES) as usize;
        flash.bytes[at..at + bytes.len()].copy_from_slice(bytes);
    }

    fn state(flash: &MemoryFlash, sector: u32) -> u32 {
        word(&flash.bytes[(OTADATA.offset + sector * SECTOR_BYTES) as usize..], 24)
    }

    #[test]
    fn the_crc_matches_esp_idf() {
        // binascii.crc32(struct.pack("<I", 1), 0xFFFFFFFF), as esptool's otatool computes it.
        assert_eq!(sequence_crc(1), 0x4743_989A);
    }

    #[test]
    fn blank_otadata_has_nothing_to_confirm() {
        let mut flash = MemoryFlash::new(0x4000);
        assert_eq!(confirm(&mut flash, OTADATA), Ok(Confirmed::Nothing));
        assert!(flash.erases.is_empty());
    }

    #[test]
    fn a_pending_image_is_confirmed_in_its_own_sector() {
        let mut flash = MemoryFlash::new(0x4000);
        put(&mut flash, 0, &entry(1, STATE_VALID));
        put(&mut flash, 1, &entry(2, STATE_PENDING_VERIFY));
        assert_eq!(confirm(&mut flash, OTADATA), Ok(Confirmed::Now));
        assert_eq!((state(&flash, 0), state(&flash, 1)), (STATE_VALID, STATE_VALID));
        assert_eq!(word(&flash.bytes[0x3000..], 0), 2, "the sequence stays");
        assert_eq!(confirm(&mut flash, OTADATA), Ok(Confirmed::Nothing), "only once");
    }

    #[test]
    fn an_aborted_newer_entry_is_left_alone() {
        // The bootloader rolled back: the newer entry is ABORTED and the older one runs.
        let mut flash = MemoryFlash::new(0x4000);
        put(&mut flash, 0, &entry(1, STATE_VALID));
        put(&mut flash, 1, &entry(2, 4));
        assert_eq!(confirm(&mut flash, OTADATA), Ok(Confirmed::Nothing));
    }

    #[test]
    fn a_corrupt_entry_is_ignored() {
        let mut flash = MemoryFlash::new(0x4000);
        let mut bad = entry(5, STATE_NEW);
        bad[28] ^= 1;
        put(&mut flash, 1, &bad);
        put(&mut flash, 0, &entry(1, STATE_NEW));
        assert_eq!(confirm(&mut flash, OTADATA), Ok(Confirmed::Now));
        assert_eq!(state(&flash, 0), STATE_VALID);
    }
}
