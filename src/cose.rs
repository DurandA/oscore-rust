use alloc::vec::Vec;
use ed25519_dalek::{Keypair, Signature};
use serde_bytes::{ByteBuf, Bytes};
use sha2::Sha512;

use crate::{cbor, Result};

/// Returns the signature from signing the `Sig_structure` of the given data.
///
/// # Arguments
/// * `id_cred_x` - The CBOR encoded header map identifying a public
///   authentication key, e.g. `{ 4 : h'1111' }`.
/// * `th_i` - The transcript hash.
/// * `cred_x` - Encoded `COSE_Key`.
/// * `keypair_bytes` - The ed25519 authentication key pair. First 32 bytes are
///   the secret key, the other 32 bytes the public key.
pub fn sign(
    id_cred_x: &[u8],
    th_i: &[u8],
    cred_x: &[u8],
    keypair_bytes: &[u8],
) -> Result<[u8; 64]> {
    let to_be_signed = build_to_be_signed(id_cred_x, th_i, cred_x)?;
    let keypair = Keypair::from_bytes(&keypair_bytes)?;
    let signature = keypair.sign::<Sha512>(&to_be_signed);

    Ok(signature.to_bytes())
}

/// Checks if the signature was made on a `Sig_structure` of the given data,
/// with the given key.
///
/// # Arguments
/// * `id_cred_x` - The CBOR encoded header map identifying a public
///   authentication key, e.g. `{ 4 : h'1111' }`.
/// * `th_i` - The transcript hash.
/// * `cred_x` - Encoded `COSE_Key`.
/// * `public_key` - The ed25519 public key of the pair used for the signature.
/// * `signature` - The ed25519 signature.
pub fn verify(
    id_cred_x: &[u8],
    th_i: &[u8],
    cred_x: &[u8],
    public_key: &[u8],
    signature: &[u8],
) -> Result<()> {
    let to_be_signed = build_to_be_signed(id_cred_x, th_i, cred_x)?;
    let public_key = ed25519_dalek::PublicKey::from_bytes(public_key)?;
    let signature = Signature::from_bytes(signature)?;

    Ok(public_key.verify::<Sha512>(&to_be_signed, &signature)?)
}

/// Returns the COSE `Sig_structure` used as input to the signature algorithm.
fn build_to_be_signed(
    id_cred_x: &[u8],
    th_i: &[u8],
    cred_x: &[u8],
) -> Result<Vec<u8>> {
    // Create the Sig_structure
    let sig_struct = (
        "Signature1",
        Bytes::new(id_cred_x), // protected
        Bytes::new(th_i),      // external_aad
        Bytes::new(cred_x),    // payload
    );

    cbor::encode(&sig_struct)
}

/// Returns a CBOR encoded `COSE_KDF_Context`.
///
/// This is used as the info input for the HKDF-Expand step.
///
/// # Arguments
/// * `algorithm_id` - The algorithm name, e.g. AES-CCM-64-64-128.
/// * `key_data_length` - The desired key length in bits.
/// * `other` - Typically a transcript hash.
pub fn build_kdf_context(
    algorithm_id: &str,
    key_data_length: usize,
    other: &[u8],
) -> Result<Vec<u8>> {
    // (keyDataLength, protected, other)
    let supp_pub_info = (key_data_length, Bytes::new(&[]), Bytes::new(other));
    // (AlgorithmID, PartyUInfo, PartyVInfo, SuppPubInfo)
    let cose_kdf_context = (algorithm_id, [(); 3], [(); 3], supp_pub_info);

    cbor::encode(cose_kdf_context)
}

/// An Octet Key Pair (OKP) `COSE_Key`.
#[derive(Debug, PartialEq)]
pub struct CoseKey {
    crv: usize,
    x: Vec<u8>,
    kty: usize,
    kid: Vec<u8>,
}

/// Returns the CBOR encoded `COSE_Key` for the given data.
///
/// This is specific to our use case where we only have X25519 public keys,
/// which are Octet Key Pairs (OKP) in COSE and represented as a single
/// x-coordinate.
pub fn serialize_cose_key(x: &[u8], kid: &[u8]) -> Result<Vec<u8>> {
    // Pack the data into a structure that nicely serializes almost into
    // what we want to have as the actual bytes for the COSE_Key.
    // (crv key, crv value, x-coordinate key, x-coordinate value,
    //  kty key, kty value, kid key, kid value)
    let raw_key = (-1, 4, -2, Bytes::new(x), 1, 1, 2, Bytes::new(kid));
    // Get the byte representation of it
    let mut bytes = cbor::encode(raw_key)?;
    // This is a CBOR array, but we want a map
    cbor::array_to_map(&mut bytes)?;

    Ok(bytes)
}

/// Returns the `COSE_Key` structure deserialized from the given bytes.
pub fn deserialize_cose_key(mut bytes: Vec<u8>) -> Result<CoseKey> {
    // Turn the CBOR map into an array that we can deserialize
    cbor::map_to_array(&mut bytes)?;
    // Try to deserialize into our raw format
    let raw_key: (isize, usize, isize, ByteBuf, isize, usize, isize, ByteBuf) =
        cbor::decode(&mut bytes)?;

    // On success, just move the items into the "nice" key structure
    Ok(CoseKey {
        crv: raw_key.1,
        x: raw_key.3.into_vec(),
        kty: raw_key.5,
        kid: raw_key.7.into_vec(),
    })
}

/// Returns the COSE header map for the given `kid`.
pub fn build_id_cred_x(kid: &[u8]) -> Result<Vec<u8>> {
    // Pack the data into a structure that nicely serializes almost into
    // what we want to have as the actual bytes for the COSE header map.
    // (kid key, kid value)
    let id_cred_x = (4, Bytes::new(kid));
    // Get the byte representation of it
    let mut bytes = cbor::encode(id_cred_x)?;
    // This is a CBOR array, but we want a map
    cbor::array_to_map(&mut bytes)?;

    Ok(bytes)
}

/// Returns the `kid` from the COSE header map.
pub fn get_kid(mut id_cred_x: Vec<u8>) -> Result<Vec<u8>> {
    // Turn the CBOR map into an array that we can deserialize
    cbor::map_to_array(&mut id_cred_x)?;
    // Try to deserialize into our raw format
    let id_cred_x: (usize, ByteBuf) = cbor::decode(&mut id_cred_x)?;

    Ok(id_cred_x.1.into_vec())
}

/// Returns the `COSE_Encrypt0` structure used as associated data in the AEAD.
pub fn build_ad(th_i: &[u8]) -> Result<Vec<u8>> {
    cbor::encode(("Encrypt0", Bytes::new(&[]), Bytes::new(th_i)))
}

#[cfg(test)]
mod tests {
    use super::*;

    static ID_CRED_X: [u8; 5] = [0xA1, 0x04, 0x42, 0x11, 0x11];
    static TH_I: [u8; 3] = [0x22, 0x22, 0x22];
    static CRED_X: [u8; 4] = [0x55, 0x55, 0x55, 0x55];
    static M: [u8; 27] = [
        0x84, 0x6a, 0x53, 0x69, 0x67, 0x6e, 0x61, 0x74, 0x75, 0x72, 0x65,
        0x31, 0x45, 0xA1, 0x04, 0x42, 0x11, 0x11, 0x43, 0x22, 0x22, 0x22,
        0x44, 0x55, 0x55, 0x55, 0x55,
    ];

    static SIGNATURE: [u8; 64] = [
        0x51, 0xA9, 0xD7, 0xCA, 0x97, 0x8E, 0x09, 0x41, 0x5A, 0xC3, 0x76,
        0x28, 0x46, 0x27, 0x12, 0xAC, 0x9D, 0xA9, 0xBD, 0xF3, 0x68, 0x2F,
        0xC4, 0x47, 0xB3, 0x06, 0x5E, 0x1B, 0x1E, 0x92, 0xAA, 0x4C, 0x3B,
        0x03, 0x95, 0x02, 0x9D, 0x6C, 0xF9, 0xF7, 0xF6, 0x73, 0x4F, 0x7C,
        0xEC, 0xE0, 0x3B, 0xAB, 0x71, 0xDB, 0x90, 0x2B, 0xC3, 0x9D, 0xA5,
        0x1B, 0x8D, 0xB7, 0x34, 0xCD, 0xD9, 0x87, 0x99, 0x06,
    ];
    static KEYPAIR: [u8; 64] = [
        0xF4, 0x20, 0x6A, 0x9E, 0xFA, 0x0A, 0xF5, 0xEF, 0x1F, 0x66, 0x88,
        0xBC, 0xAF, 0xDA, 0xF8, 0x16, 0x0C, 0xC5, 0x88, 0x54, 0x5C, 0x24,
        0x08, 0xF1, 0x8C, 0xAF, 0x8C, 0x8F, 0xA6, 0xE7, 0x67, 0x75, 0xAA,
        0x71, 0xD1, 0xFE, 0xB3, 0xD7, 0xD7, 0x8C, 0x14, 0x7F, 0xBD, 0xCA,
        0xAD, 0x34, 0x67, 0x88, 0xC2, 0x44, 0x32, 0x3E, 0xC6, 0x4D, 0x9A,
        0x85, 0x68, 0x6D, 0x4D, 0x06, 0xA9, 0x58, 0x6F, 0x20,
    ];

    #[test]
    fn to_be_signed() {
        let to_be_signed =
            build_to_be_signed(&ID_CRED_X, &TH_I, &CRED_X).unwrap();
        assert_eq!(&M[..], &to_be_signed[..]);
    }

    #[test]
    fn signature_same() {
        let signature = sign(&ID_CRED_X, &TH_I, &CRED_X, &KEYPAIR).unwrap();
        assert_eq!(&SIGNATURE[..], &signature[..]);
    }

    #[test]
    fn signature_verifies() {
        let signature = sign(&ID_CRED_X, &TH_I, &CRED_X, &KEYPAIR).unwrap();
        assert!(verify(
            &ID_CRED_X,
            &TH_I,
            &CRED_X,
            &KEYPAIR[32..],
            &signature
        )
        .is_ok());

        let mut cred_x_changed = CRED_X.to_vec();
        cred_x_changed[1] = 0x44;
        assert!(verify(
            &ID_CRED_X,
            &TH_I,
            &cred_x_changed,
            &KEYPAIR[32..],
            &signature
        )
        .is_err());
    }

    static ALG1: &str = "IV-GENERATION";
    static ALG2: &str = "AES-CCM-64-64-128";
    static LEN1: usize = 104;
    static LEN2: usize = 128;
    static OTHER: [u8; 2] = [0xAA, 0xAA];
    static CONTEXT1: [u8; 30] = [
        0x84, 0x6D, 0x49, 0x56, 0x2D, 0x47, 0x45, 0x4E, 0x45, 0x52, 0x41,
        0x54, 0x49, 0x4F, 0x4E, 0x83, 0xF6, 0xF6, 0xF6, 0x83, 0xF6, 0xF6,
        0xF6, 0x83, 0x18, 0x68, 0x40, 0x42, 0xAA, 0xAA,
    ];
    static CONTEXT2: [u8; 34] = [
        0x84, 0x71, 0x41, 0x45, 0x53, 0x2D, 0x43, 0x43, 0x4D, 0x2D, 0x36,
        0x34, 0x2D, 0x36, 0x34, 0x2D, 0x31, 0x32, 0x38, 0x83, 0xF6, 0xF6,
        0xF6, 0x83, 0xF6, 0xF6, 0xF6, 0x83, 0x18, 0x80, 0x40, 0x42, 0xAA,
        0xAA,
    ];

    #[test]
    fn context_generation() {
        let context_bytes = build_kdf_context(ALG1, LEN1, &OTHER).unwrap();
        assert_eq!(&CONTEXT1[..], &context_bytes[..]);

        let context_bytes = build_kdf_context(ALG2, LEN2, &OTHER).unwrap();
        assert_eq!(&CONTEXT2[..], &context_bytes[..]);
    }

    static CURVE: usize = 4;
    static X: [u8; 4] = [0x00, 0x01, 0x02, 0x03];
    static KTY: usize = 1;
    static KID: [u8; 4] = [0x04, 0x05, 0x06, 0x07];
    static KEY_BYTES: [u8; 17] = [
        0xA4, 0x20, 0x04, 0x21, 0x44, 0x00, 0x01, 0x02, 0x03, 0x01, 0x01,
        0x02, 0x44, 0x04, 0x05, 0x06, 0x07,
    ];

    #[test]
    fn key_encode() {
        assert_eq!(&KEY_BYTES[..], &serialize_cose_key(&X, &KID).unwrap()[..]);
    }

    #[test]
    fn key_decode() {
        let key = CoseKey {
            crv: CURVE,
            x: X.to_vec(),
            kty: KTY,
            kid: KID.to_vec(),
        };
        let bytes = KEY_BYTES.to_vec();

        assert_eq!(key, deserialize_cose_key(bytes).unwrap());
    }

    static KID_2: [u8; 2] = [0x00, 0x01];
    static ID_CRED_X_2: [u8; 5] = [0xA1, 0x04, 0x42, 0x00, 0x01];

    #[test]
    fn encode_id_cred_x() {
        let bytes = build_id_cred_x(&KID_2).unwrap();
        assert_eq!(&ID_CRED_X_2[..], &bytes[..]);
    }

    #[test]
    fn decode_id_cred_x() {
        let kid = get_kid(ID_CRED_X_2.to_vec()).unwrap();
        assert_eq!(&KID_2[..], &kid[..]);
    }

    static ENCRYPT_0_TH: [u8; 3] = [0x01, 0x02, 0x03];
    static ENCRYPT_0: [u8; 15] = [
        0x83, 0x68, 0x45, 0x6e, 0x63, 0x72, 0x79, 0x70, 0x74, 0x30, 0x40,
        0x43, 0x01, 0x02, 0x03,
    ];

    #[test]
    fn encrypt_0() {
        assert_eq!(&ENCRYPT_0[..], &build_ad(&ENCRYPT_0_TH).unwrap()[..]);
    }
}
