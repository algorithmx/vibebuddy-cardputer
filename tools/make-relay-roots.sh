#!/usr/bin/env bash
# Rebuild firmware-rs/device/certs/relay-roots.pem: the roots the box trusts for the Relay's TLS.
# The Relay sits on Cloudflare's Universal SSL, whose CA can be Let's Encrypt, Google Trust Services or
# SSL.com, with a backup certificate from Sectigo deployed on revocation (#110). Each CA's ECDSA and RSA
# roots are taken from macOS's system root store. Replacing the bundle needs a firmware release.
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
out="${repo_root}/firmware-rs/device/certs/relay-roots.pem"
store=/System/Library/Keychains/SystemRootCertificates.keychain
roots=(
    "GTS Root R1"
    "GTS Root R4"
    "ISRG Root X1"
    "ISRG Root X2"
    "SSL.com TLS ECC Root CA 2022"
    "SSL.com TLS RSA Root CA 2022"
    "SSL.com Root Certification Authority ECC"
    "SSL.com Root Certification Authority RSA"
    "Sectigo Public Server Authentication Root E46"
    "Sectigo Public Server Authentication Root R46"
    "USERTrust ECC Certification Authority"
    "USERTrust RSA Certification Authority"
)

mkdir -p "$(dirname "${out}")"
: > "${out}"
for name in "${roots[@]}"; do
    pem="$(security find-certificate -a -c "${name}" -p "${store}")"
    if [[ "$(grep -c 'BEGIN CERTIFICATE' <<< "${pem}")" -ne 1 ]]; then
        echo "expected exactly one root named ${name}" >&2
        exit 1
    fi
    printf '# %s\n%s\n' "${name}" "${pem}" >> "${out}"
done
echo "${#roots[@]} roots -> ${out}"
