use bls12_381_plus::{ExpandMsg, G1Affine, G1Projective, G2Affine, G2Projective, Scalar};

use crate::{
    ciphersuite::{BbsCiphersuite, POINT_LEN},
    encoding::I2OSP,
};

// https://www.ietf.org/archive/id/draft-irtf-cfrg-bbs-signatures-00.html#name-mapmessagetoscalarashash
#[allow(unused_comparisons)]
pub(crate) fn map_message_to_scalar_as_hash<'a, T>(message: &[u8], dst: &[u8]) -> Scalar
where
    T: BbsCiphersuite<'a>,
{
    let dst = if dst.is_empty() {
        T::map_msg_to_scalar_as_hash_dst()
    } else {
        dst.into()
    };

    // 1. msg_for_hash = encode_for_hash(msg)
    let msg_for_hash = message.encode_for_hash();

    // 2. if msg_for_hash is INVALID, return INVALID
    // 3. if length(dst) > 255, return INVALID
    // 4. return hash_to_scalar(msg_for_hash, 1, dst)
    let mut result = [Scalar::zero(); 1];
    hash_to_scalar::<T>(&msg_for_hash, &dst, &mut result);

    result[0]
}

// https://www.ietf.org/archive/id/draft-irtf-cfrg-bbs-signatures-00.html#section-4.3
pub(crate) fn hash_to_scalar<'a, T>(msg_octets: &[u8], dst: &[u8], out: &mut [Scalar])
where
    T: BbsCiphersuite<'a>,
{
    let dst = if dst.is_empty() {
        T::hash_to_scalar_dst()
    } else {
        dst.into()
    };
    let count = out.len();

    // 1.  len_in_bytes = count * expand_len
    let len_in_bytes = count * POINT_LEN;

    // 2.  t = 0
    let mut t = 0usize;

    // 3.  msg_prime = msg_octets || I2OSP(t, 1) || I2OSP(count, 4)
    let msg_prime = [msg_octets, &t.i2osp(1), &count.i2osp(4)].concat();

    // 4.  uniform_bytes = expand_message(msg_prime, h2s_dst, len_in_bytes)
    let mut uniform_bytes = vec![0u8; len_in_bytes];
    T::Expander::expand_message(&msg_prime, &dst, &mut uniform_bytes);

    // 5.  for i in (1, ..., count):
    // 6.      tv = uniform_bytes[(i-1)*expand_len..i*expand_len-1]
    // 7.      scalar_i = OS2IP(tv) mod r
    // 8.  if 0 in (scalar_1, ..., scalar_count):
    // 9.      t = t + 1
    // 10.     go back to step 3
    let mut i = 0;
    for item in out {
        *item = Scalar::from_okm(
            uniform_bytes[i * POINT_LEN..(i + 1) * POINT_LEN]
                .try_into()
                .unwrap(),
        );
        i += 1;
    }
}

pub trait EncodeForHash {
    fn encode_for_hash(&self) -> Vec<u8>;
}

impl EncodeForHash for dyn AsRef<[u8]> {
    fn encode_for_hash(&self) -> Vec<u8> {
        self.as_ref().into()
    }
}

impl EncodeForHash for &str {
    fn encode_for_hash(&self) -> Vec<u8> {
        self.as_bytes().encode_for_hash()
    }
}

impl EncodeForHash for &[u8] {
    fn encode_for_hash(&self) -> Vec<u8> {
        [self.len().i2osp(8), self.to_vec()].concat()
    }
}

impl EncodeForHash for usize {
    fn encode_for_hash(&self) -> Vec<u8> {
        self.i2osp(8)
    }
}

impl EncodeForHash for u64 {
    fn encode_for_hash(&self) -> Vec<u8> {
        self.i2osp(8)
    }
}

impl EncodeForHash for u8 {
    fn encode_for_hash(&self) -> Vec<u8> {
        (*self as u64).i2osp(8)
    }
}

impl EncodeForHash for G2Projective {
    fn encode_for_hash(&self) -> Vec<u8> {
        G2Affine::from(self).to_compressed().to_vec()
    }
}

impl EncodeForHash for G1Projective {
    fn encode_for_hash(&self) -> Vec<u8> {
        G1Affine::from(self).to_compressed().to_vec()
    }
}

impl EncodeForHash for G2Affine {
    fn encode_for_hash(&self) -> Vec<u8> {
        self.to_compressed().to_vec()
    }
}

impl EncodeForHash for G1Affine {
    fn encode_for_hash(&self) -> Vec<u8> {
        self.to_compressed().to_vec()
    }
}

impl EncodeForHash for Scalar {
    fn encode_for_hash(&self) -> Vec<u8> {
        self.i2osp(32)
    }
}

#[cfg(test)]
mod test {
    use bls12_381_plus::Scalar;
    use fluid::prelude::*;

    use crate::{ciphersuite::*, encoding::*, fixture, from_hex, hashing::*, tests::*};

    #[test]
    fn test_encode() {
        let s = "hello world".encode_for_hash();
        assert_eq!(11 + 8, s.len());
    }

    #[theory]
    #[case("bls12-381-sha-256/MapMessageToScalarAsHash.json", Bls12381Sha256)]
    #[case("bls12-381-shake-256/MapMessageToScalarAsHash.json", Bls12381Shake256)]
    fn map_message_to_scalar_test<'a, T>(file: &str, _: T)
    where
        T: BbsCiphersuite<'a>,
    {
        let input = fixture!(MapMessageToScalar, file);

        for c in input.cases {
            assert_eq!(
                map_message_to_scalar_as_hash::<T>(
                    &from_hex!(c.message),
                    &from_hex!(input.dst.as_bytes())
                ),
                Scalar::os2ip(&from_hex!(c.scalar))
            );
        }
    }

    #[theory]
    #[case("bls12-381-sha-256/h2s/h2s001.json", Bls12381Sha256)]
    // #[case("bls12-381-sha-256/h2s/h2s002.json", Bls12381Sha256)]
    #[case("bls12-381-shake-256/h2s/h2s001.json", Bls12381Shake256)]
    #[case("bls12-381-shake-256/h2s/h2s002.json", Bls12381Shake256)]
    fn hash_to_scalar_test<'a, T>(file: &str, _: T)
    where
        T: BbsCiphersuite<'a>,
    {
        let input = fixture!(HashToScalar, file);

        let mut actual = vec![Scalar::zero(); input.count];
        hash_to_scalar::<T>(
            &from_hex!(input.message),
            &from_hex!(input.dst),
            &mut actual,
        );

        assert_eq!(actual.len(), input.scalars.len());

        for i in 0..input.scalars.len() {
            assert_eq!(
                Scalar::os2ip(&from_hex!(input.scalars[i].as_bytes())),
                actual[i]
            );
        }
    }
}
