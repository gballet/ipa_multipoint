use ark_ff::One;
use ark_std::rand;
use ark_std::rand::SeedableRng;
use ark_std::UniformRand;
use bandersnatch::{EdwardsProjective, Fr};
use criterion::{black_box, criterion_group, criterion_main, BatchSize, Criterion};
use ipa_bandersnatch::ipa::create;
use ipa_bandersnatch::lagrange_basis::*;
use ipa_bandersnatch::math_utils::inner_product;
use ipa_bandersnatch::multiproof::*;
use ipa_bandersnatch::slow_vartime_multiscalar_mul;
use ipa_bandersnatch::transcript::TranscriptProtocol;
use merlin::Transcript;
use rand_chacha::ChaCha20Rng;
use std::iter;

use criterion::BenchmarkId;

pub fn criterion_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("verify multiopen-deg-256");

    use ark_std::test_rng;

    // Setup parameters, n is the degree + 1
    // CRs is the G_Vec, H_Vec, Q group elements
    let n = 256;
    let crs = CRS::new(n);

    let mut rng = test_rng();
    let poly = LagrangeBasis::new((0..n).map(|_| Fr::rand(&mut rng)).collect());
    let poly_comm = crs.commit_lagrange_poly(&poly);

    for num_polynomials in [10_000, 20_000] {
        // For verification, we simply generate one polynomial and then clone it `num_polynomial`
        // time.  whether it is the same polynomial or different polynomial does not affect verification.
        let mut polys: Vec<LagrangeBasis> = Vec::with_capacity(num_polynomials);
        for _ in 0..num_polynomials {
            polys.push(poly.clone())
        }

        let mut prover_queries = Vec::with_capacity(num_polynomials);
        for poly in polys.into_iter() {
            let point = 1;
            let y_i = poly.evaluate_in_domain(point);

            let prover_query = ProverQueryLagrange {
                comm: poly_comm.clone(),
                poly: poly,
                x_i: point,
                y_i,
            };

            prover_queries.push(prover_query);
        }

        let precomp = PrecomputedWeights::new(n);

        let mut transcript = Transcript::new(b"foo");
        let multiproof = MultiOpen::open_multiple_lagrange(
            &crs,
            &precomp,
            &mut transcript,
            prover_queries.clone(),
        );

        let mut transcript = Transcript::new(b"foo");
        let mut verifier_queries: Vec<VerifierQuery> = Vec::with_capacity(num_polynomials);
        for prover_query in prover_queries {
            verifier_queries.push(prover_query.into())
        }

        group.bench_with_input(
            BenchmarkId::from_parameter(num_polynomials),
            &num_polynomials,
            |b, _| {
                b.iter_batched(
                    || transcript.clone(),
                    |mut transcript| {
                        multiproof.check_single_lagrange(
                            &crs,
                            &precomp,
                            &verifier_queries,
                            &mut transcript,
                            n,
                        )
                    },
                    BatchSize::SmallInput,
                )
            },
        );
    }

    group.finish();
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
