/*!
`crypto_secretbox_xsalsa20poly1305`, a particular
combination of Salsa20 and Poly1305 specified in
[Cryptography in NaCl](http://nacl.cr.yp.to/valid.html).

This function is conjectured to meet the standard notions of privacy and
authenticity.
*/
use libc::{c_ulonglong, c_int};
use std::intrinsics::volatile_set_memory;
use utils::marshal;
use randombytes::randombytes_into;

#[link(name = "sodium")]
extern {
    fn crypto_secretbox_xsalsa20poly1305(c: *mut u8,
                                         m: *const u8,
                                         mlen: c_ulonglong,
                                         n: *const u8,
                                         k: *const u8) -> c_int;
    fn crypto_secretbox_xsalsa20poly1305_open(m: *mut u8,
                                              c: *const u8,
                                              clen: c_ulonglong,
                                              n: *const u8,
                                              k: *const u8) -> c_int;
}

pub const KEYBYTES: uint = 32;
pub const NONCEBYTES: uint = 24;

/**
 * `Key` for symmetric authenticated encryption
 *
 * When a `Key` goes out of scope its contents
 * will be zeroed out
 */
pub struct Key(pub [u8, ..KEYBYTES]);

newtype_drop!(Key)
newtype_clone!(Key)

/**
 * `Nonce` for symmetric authenticated encryption
 */
pub struct Nonce(pub [u8, ..NONCEBYTES]);

newtype_clone!(Nonce)

pub const ZERO: [u8, ..32] = [0, ..32];
pub const BOXZERO: [u8, ..16] = [0, ..16];

/**
 * `gen_key()` randomly generates a secret key
 *
 * THREAD SAFETY: `gen_key()` is thread-safe provided that you have
 * called `sodiumoxide::init()` once before using any other function
 * from sodiumoxide.
 */
pub fn gen_key() -> Key {
    let mut key = [0, ..KEYBYTES];
    randombytes_into(key);
    Key(key)
}

/**
 * `gen_nonce()` randomly generates a nonce
 *
 * THREAD SAFETY: `gen_key()` is thread-safe provided that you have
 * called `sodiumoxide::init()` once before using any other function
 * from sodiumoxide.
 */
pub fn gen_nonce() -> Nonce {
    let mut nonce = [0, ..NONCEBYTES];
    randombytes_into(nonce);
    Nonce(nonce)
}

/**
 * `seal()` encrypts and authenticates a message `m` using a secret key `k` and a
 * nonce `n`.  It returns a ciphertext `c`.
 */
pub fn seal(m: &[u8],
            n: &Nonce,
            k: &Key) -> Vec<u8> {
    marshal(m, ZERO, |b| {
        seal_inplace(b.as_mut_slice(), n, k)
    }).unwrap()
}

/**
 * `seal_inplace()` encrypts and authenticates a message `m` using a secret key `k` and a
 * nonce `n`.  It returns a ciphertext `c`.
 *
 * `seal_inplace()` requires that the first `ZERO.len()` bytes of the
 * message are equal to 0, otherwise it returns `None`.
 * `seal_inplace()` will modify the message in place, but returns a
 * slice pointing to the start of the actual ciphertext (minus
 * padding).
 */
pub fn seal_inplace<'a>(m: &'a mut [u8],
                        &Nonce(n): &Nonce,
                        &Key(k): &Key) -> Option<&'a [u8]> {
    if m.slice_to(ZERO.len()) != ZERO {
        return None
    } 

    unsafe {
        crypto_secretbox_xsalsa20poly1305(m.as_mut_ptr(),
                                          m.as_ptr(),
                                          m.len() as c_ulonglong,
                                          n.as_ptr(),
                                          k.as_ptr());
    }
    Some(m.slice_from(BOXZERO.len()))
}

/**
 * `open()` verifies and decrypts a ciphertext `c` using a secret key
 * `k` and a nonce `n`.
 * It returns a plaintext `Some(m)`.
 * If the ciphertext fails verification, `open()` returns `None`.
 */
pub fn open(c: &[u8],
            n: &Nonce,
            k: &Key) -> Option<Vec<u8>> {
    marshal(c, BOXZERO, |b| {
        open_inplace(b.as_mut_slice(), n, k)
    })
}

/**
 * `open_inplace()` verifies and decrypts a ciphertext `c` using a secret key
 * `k` and a nonce `n`.  It returns a plaintext `Some(m)`.  If the
 * ciphertext fails verification, `open_inplace()` returns `None`.
 *
 * `open_inplace()` requires that the first `BOXZERO.len()` bytes of the ciphertext 
 * are equal to 0, otherwise it returns `None`.
 * `open_inplace()` will modify the ciphertext in place, but returns a slice
 * pointing to the start of the actual plaintext (minus padding).
 */
pub fn open_inplace<'a>(c: &'a mut [u8],
                        &Nonce(n): &Nonce,
                        &Key(k): &Key) -> Option<&'a [u8]> {
    if c.slice_to(BOXZERO.len()) != BOXZERO {
        return None
    }

    unsafe {
        let ret = crypto_secretbox_xsalsa20poly1305_open(c.as_mut_ptr(),
                                                         c.as_ptr(),
                                                         c.len() as c_ulonglong,
                                                         n.as_ptr(),
                                                         k.as_ptr());
        if ret == 0 {
            Some(c.slice_from(ZERO.len()))
        } else {
            None
        }
    }
}
                        
#[test]
fn test_seal_open() {
    use randombytes::randombytes;
    for i in range(0, 256u) {
        let k = gen_key();
        let m = randombytes(i);
        let n = gen_nonce();
        let c = seal(m.as_slice(), &n, &k);
        let opened = open(c.as_slice(), &n, &k);
        assert!(Some(m) == opened);
    }
}

#[test]
fn test_seal_open_tamper() {
    use randombytes::randombytes;
    for i in range(0, 32u) {
        let k = gen_key();
        let m = randombytes(i);
        let n = gen_nonce();
        let mut cv = seal(m.as_slice(), &n, &k);
        let c = cv.as_mut_slice();
        for i in range(0, c.len()) {
            c[i] ^= 0x20;
            assert!(None == open(c, &n, &k));
            c[i] ^= 0x20;
        }
    }
}

#[test]
fn test_vector_1() {
    let firstkey = Key([0x1b,0x27,0x55,0x64,0x73,0xe9,0x85,0xd4
                       ,0x62,0xcd,0x51,0x19,0x7a,0x9a,0x46,0xc7
                       ,0x60,0x09,0x54,0x9e,0xac,0x64,0x74,0xf2
                       ,0x06,0xc4,0xee,0x08,0x44,0xf6,0x83,0x89]);
    let nonce = Nonce([0x69,0x69,0x6e,0xe9,0x55,0xb6,0x2b,0x73
                      ,0xcd,0x62,0xbd,0xa8,0x75,0xfc,0x73,0xd6
                      ,0x82,0x19,0xe0,0x03,0x6b,0x7a,0x0b,0x37]);
    let m = vec![0xbe,0x07,0x5f,0xc5,0x3c,0x81,0xf2,0xd5
             ,0xcf,0x14,0x13,0x16,0xeb,0xeb,0x0c,0x7b
             ,0x52,0x28,0xc5,0x2a,0x4c,0x62,0xcb,0xd4
             ,0x4b,0x66,0x84,0x9b,0x64,0x24,0x4f,0xfc
             ,0xe5,0xec,0xba,0xaf,0x33,0xbd,0x75,0x1a
             ,0x1a,0xc7,0x28,0xd4,0x5e,0x6c,0x61,0x29
             ,0x6c,0xdc,0x3c,0x01,0x23,0x35,0x61,0xf4
             ,0x1d,0xb6,0x6c,0xce,0x31,0x4a,0xdb,0x31
             ,0x0e,0x3b,0xe8,0x25,0x0c,0x46,0xf0,0x6d
             ,0xce,0xea,0x3a,0x7f,0xa1,0x34,0x80,0x57
             ,0xe2,0xf6,0x55,0x6a,0xd6,0xb1,0x31,0x8a
             ,0x02,0x4a,0x83,0x8f,0x21,0xaf,0x1f,0xde
             ,0x04,0x89,0x77,0xeb,0x48,0xf5,0x9f,0xfd
             ,0x49,0x24,0xca,0x1c,0x60,0x90,0x2e,0x52
             ,0xf0,0xa0,0x89,0xbc,0x76,0x89,0x70,0x40
             ,0xe0,0x82,0xf9,0x37,0x76,0x38,0x48,0x64
             ,0x5e,0x07,0x05];

    let c_expected = vec![0xf3,0xff,0xc7,0x70,0x3f,0x94,0x00,0xe5
                      ,0x2a,0x7d,0xfb,0x4b,0x3d,0x33,0x05,0xd9
                      ,0x8e,0x99,0x3b,0x9f,0x48,0x68,0x12,0x73
                      ,0xc2,0x96,0x50,0xba,0x32,0xfc,0x76,0xce
                      ,0x48,0x33,0x2e,0xa7,0x16,0x4d,0x96,0xa4
                      ,0x47,0x6f,0xb8,0xc5,0x31,0xa1,0x18,0x6a
                      ,0xc0,0xdf,0xc1,0x7c,0x98,0xdc,0xe8,0x7b
                      ,0x4d,0xa7,0xf0,0x11,0xec,0x48,0xc9,0x72
                      ,0x71,0xd2,0xc2,0x0f,0x9b,0x92,0x8f,0xe2
                      ,0x27,0x0d,0x6f,0xb8,0x63,0xd5,0x17,0x38
                      ,0xb4,0x8e,0xee,0xe3,0x14,0xa7,0xcc,0x8a
                      ,0xb9,0x32,0x16,0x45,0x48,0xe5,0x26,0xae
                      ,0x90,0x22,0x43,0x68,0x51,0x7a,0xcf,0xea
                      ,0xbd,0x6b,0xb3,0x73,0x2b,0xc0,0xe9,0xda
                      ,0x99,0x83,0x2b,0x61,0xca,0x01,0xb6,0xde
                      ,0x56,0x24,0x4a,0x9e,0x88,0xd5,0xf9,0xb3
                      ,0x79,0x73,0xf6,0x22,0xa4,0x3d,0x14,0xa6
                      ,0x59,0x9b,0x1f,0x65,0x4c,0xb4,0x5a,0x74
                      ,0xe3,0x55,0xa5];
    let c = seal(m.as_slice(), &nonce, &firstkey);
    assert!(c == c_expected);
    let m2 = open(c.as_slice(), &nonce, &firstkey);
    assert!(Some(m) == m2);
}