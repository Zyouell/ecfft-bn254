use std::convert::TryInto;

use crate::{ecfft::EcFftParameters, utils::isogeny::Isogeny};
use ark_ff::BigInteger384;

type F = ark_bls12_381::Fq;
/// Number of 64-bit limbs needed to represent field elements.
const NUM_LIMBS: usize = 6;

/// ECFFT parameters for the BLS12-381 base field `F`.
/// Computed with the curve `E = EllipticCurve(F, [a, b])` with
/// `a, b = 0x287cc81c41f14f729fcbc12f57b2dd49bdcfc64938f9ad946c9fe5288aa3e9653670d336b09c058baad66ae717c1df7, 0x33f44f9b6fd7ba0080f0ad4843e076da70b11e6846d41e19792a15a4920e2294f9c971db67257eefea71c70514c6e54`
pub struct Bls12381Parameters;

impl EcFftParameters<F> for Bls12381Parameters {
    /// The curve `E` has order `4002409555221667393417789825735904156556882819939007885330472032889288775404654397856791416969022033299503997812736`
    /// with factorization `2^15 * 122143846289723736371392511771725590715236902463958980875563721706826439679097119075219464629181580606063964777`
    const LOG_N: usize = 15;

    const N: usize = 1 << Self::LOG_N;

    /// Get the coset from the `bls12-381_coset` file. This file can be generated by running `get_params.sage`.
    fn coset() -> Vec<F> {
        std::fs::read_to_string("bls12-381_coset")
            .expect("Run `get_params.sage` to generate the coset.")
            .split_whitespace()
            .map(|s| s.parse().unwrap())
            .collect::<Vec<u64>>()
            .chunks(NUM_LIMBS)
            .map(|chunk| BigInteger384::new(chunk.try_into().unwrap()).into())
            .collect()
    }

    /// Get the isogenies from the `bls12-381_isogenies` file. This file can be generated by running `get_params.sage`.
    fn isogenies() -> Vec<Isogeny<F>> {
        std::fs::read_to_string("bls12-381_isogenies")
            .expect("Run `get_params.sage` to generate the coset.")
            .split_whitespace()
            .map(|s| s.parse().unwrap())
            .collect::<Vec<u64>>()
            .chunks(5 * NUM_LIMBS)
            .map(|chunk| {
                let numerator = (0..3)
                    .map(|i| {
                        BigInteger384::new(
                            chunk[i * NUM_LIMBS..(i + 1) * NUM_LIMBS]
                                .try_into()
                                .unwrap(),
                        )
                        .into()
                    })
                    .collect::<Vec<_>>()
                    .try_into()
                    .unwrap();
                let denominator = (3..5)
                    .map(|i| {
                        BigInteger384::new(
                            chunk[i * NUM_LIMBS..(i + 1) * NUM_LIMBS]
                                .try_into()
                                .unwrap(),
                        )
                        .into()
                    })
                    .collect::<Vec<_>>()
                    .try_into()
                    .unwrap();
                Isogeny {
                    numerator,
                    denominator,
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use crate::ecfft::{EcFftCosetPrecomputation, EcFftParameters, EcFftPrecomputationStep};

    use super::{Bls12381Parameters, F};
    use ark_ff::PrimeField;
    use ark_poly::{univariate::DensePolynomial, Polynomial};
    use ark_std::{
        rand::{distributions::Standard, prelude::Distribution, Rng},
        test_rng,
    };

    #[test]
    /// Tests that precomputations don't panic.
    fn test_precompute() {
        Bls12381Parameters::precompute_on_coset(&Bls12381Parameters::coset());
        Bls12381Parameters::precompute_on_coset(
            &Bls12381Parameters::coset()
                .into_iter()
                .step_by(2)
                .collect::<Vec<_>>(),
        );
    }

    /// Tests the extend function with a polynomial of degree `2^i - 1`.
    fn test_extend_i<F: PrimeField, P: EcFftParameters<F>>(
        i: usize,
        precomputation: &EcFftCosetPrecomputation<F, P>,
    ) where
        Standard: Distribution<F>,
    {
        let n = 1 << i;
        let mut rng = test_rng();
        let coeffs: Vec<F> = (0..n).map(|_| rng.gen()).collect();
        let poly = DensePolynomial { coeffs };
        let EcFftPrecomputationStep { s, s_prime, .. } =
            &precomputation.steps[Bls12381Parameters::LOG_N - 1 - i];
        let evals_s = s.iter().map(|x| poly.evaluate(x)).collect::<Vec<_>>();
        let evals_s_prime = s_prime.iter().map(|x| poly.evaluate(x)).collect::<Vec<_>>();
        assert_eq!(evals_s_prime, precomputation.extend(&evals_s));
    }

    #[test]
    /// Tests the extend function for various degrees.
    fn test_extend() {
        let precomputation = Bls12381Parameters::precompute_on_coset(&Bls12381Parameters::coset());
        for i in 1..Bls12381Parameters::LOG_N {
            test_extend_i::<F, _>(i, &precomputation);
        }
    }

    #[test]
    /// Tests the `evaluate_over_domain` function for various degrees.
    fn test_eval() {
        type P = Bls12381Parameters;
        let precomputation = P::precompute();
        for i in 0..P::LOG_N {
            let mut rng = test_rng();
            let coeffs: Vec<F> = (0..P::N >> i).map(|_| rng.gen()).collect();
            let poly = DensePolynomial { coeffs };
            let now = std::time::Instant::now();
            let evals = P::sub_coset(i)
                .iter()
                .map(|x| poly.evaluate(x))
                .collect::<Vec<_>>();
            dbg!(now.elapsed().as_secs_f32());
            assert_eq!(evals, precomputation.evaluate_over_domain(&poly));
            dbg!(now.elapsed().as_secs_f32());
        }
    }
}
