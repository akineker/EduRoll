use crate::fr::Fr;
use crate::poseidon_constants::*;

pub fn poseidon(inputs: &[Fr]) -> Fr {
    match inputs.len() {
        2 => permute::<3>(inputs, &ARK_T3, flatten_mds3(), PARTIAL_ROUNDS_T3),
        3 => permute::<4>(inputs, &ARK_T4, flatten_mds4(), PARTIAL_ROUNDS_T4),
        4 => permute::<5>(inputs, &ARK_T5, flatten_mds5(), PARTIAL_ROUNDS_T5),
        5 => permute::<6>(inputs, &ARK_T6, flatten_mds6(), PARTIAL_ROUNDS_T6),
        n => panic!("poseidon: unsupported input count {n} (2..=5 supported)"),
    }
}

pub fn poseidon2(a: &Fr, b: &Fr) -> Fr {
    poseidon(&[*a, *b])
}

pub fn poseidon3(a: &Fr, b: &Fr, c: &Fr) -> Fr {
    poseidon(&[*a, *b, *c])
}

pub fn poseidon4(a: &Fr, b: &Fr, c: &Fr, d: &Fr) -> Fr {
    poseidon(&[*a, *b, *c, *d])
}

pub fn poseidon5(a: &Fr, b: &Fr, c: &Fr, d: &Fr, e: &Fr) -> Fr {
    poseidon(&[*a, *b, *c, *d, *e])
}

fn flatten_mds3() -> &'static [[[u64; 4]; 3]] { &MDS_T3 }
fn flatten_mds4() -> &'static [[[u64; 4]; 4]] { &MDS_T4 }
fn flatten_mds5() -> &'static [[[u64; 4]; 5]] { &MDS_T5 }
fn flatten_mds6() -> &'static [[[u64; 4]; 6]] { &MDS_T6 }

fn permute<const T: usize>(
    inputs: &[Fr],
    ark: &[[u64; 4]],
    mds: &[[[u64; 4]; T]],
    partial_rounds: usize,
) -> Fr {
    debug_assert_eq!(inputs.len() + 1, T);
    let all_rounds = FULL_ROUNDS + partial_rounds;
    let half = FULL_ROUNDS / 2;

    let mut state = [Fr::ZERO; T];
    for (i, x) in inputs.iter().enumerate() {
        state[i + 1] = *x;
    }

    for round in 0..all_rounds {
        for i in 0..T {
            state[i] = state[i].add(&Fr(ark[round * T + i]));
        }
        let full = round < half || round >= half + partial_rounds;
        if full {
            for s in state.iter_mut() {
                *s = s.pow5();
            }
        } else {
            state[0] = state[0].pow5();
        }
        let mut new_state = [Fr::ZERO; T];
        for i in 0..T {
            let mut acc = Fr::ZERO;
            for j in 0..T {
                acc = acc.add(&state[j].mul(&Fr(mds[i][j])));
            }
            new_state[i] = acc;
        }
        state = new_state;
    }

    state[0]
}

#[cfg(test)]
mod tests {
    use super::*;
    use ark_bn254::Fr as ArkFr;
    use ark_ff::PrimeField;
    use light_poseidon::{Poseidon as LightPoseidon, PoseidonHasher};

    fn ark(bytes: &[u8; 32]) -> ArkFr {
        ArkFr::from_be_bytes_mod_order(bytes)
    }

    #[test]
    fn parity_with_light_poseidon_all_widths() {
        let inputs_bytes: Vec<[u8; 32]> = (1u8..=5)
            .map(|i| {
                let mut b = [0u8; 32];
                b[31] = i;
                b[0] = 0x01;
                b
            })
            .collect();

        for n in 2..=5usize {
            let ours = poseidon(
                &inputs_bytes[..n]
                    .iter()
                    .map(Fr::from_be_bytes_mod_order)
                    .collect::<Vec<_>>(),
            );

            let mut lp = LightPoseidon::<ArkFr>::new_circom(n).unwrap();
            let theirs = lp
                .hash(&inputs_bytes[..n].iter().map(|b| ark(b)).collect::<Vec<_>>())
                .unwrap();

            use ark_ff::BigInteger;
            let mut be = theirs.into_bigint().to_bytes_be();
            while be.len() < 32 {
                be.insert(0, 0);
            }
            assert_eq!(
                ours.to_be_bytes().to_vec(),
                be,
                "poseidon width mismatch for {n} inputs"
            );
        }
    }

    #[test]
    fn known_answer_vector() {
        let a = Fr::from_be_bytes_mod_order(&[1u8; 32]);
        let b = Fr::from_be_bytes_mod_order(&[2u8; 32]);
        let h = poseidon2(&a, &b);
        assert_eq!(
            h.to_be_bytes(),
            [
                13, 84, 225, 147, 143, 138, 140, 28, 125, 235, 94, 3, 85, 242, 99, 25, 32, 123,
                132, 254, 156, 162, 206, 27, 38, 231, 53, 200, 41, 130, 25, 144
            ]
        );
    }
}
