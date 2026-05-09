use skymemory_core::Metric;
use std::arch::x86_64::*;

pub fn distance(a: &[f32], b: &[f32], metric: &Metric) -> f64 {
    match metric {
        Metric::Euclidean => euclidean(a, b),
        Metric::Cosine => cosine(a, b),
        Metric::DotProduct => dot_product_distance(a, b),
    }
}

pub fn euclidean(a: &[f32], b: &[f32]) -> f64 {
    let len = a.len().min(b.len());
    let mut sum = 0.0f32;
    let mut i = 0;

    #[cfg(target_arch = "x86_64")]
    {
        if is_x86_feature_detected!("avx") && len >= 8 {
            unsafe {
                let mut acc = _mm256_setzero_ps();
                while i + 8 <= len {
                    let va = _mm256_loadu_ps(a.as_ptr().add(i));
                    let vb = _mm256_loadu_ps(b.as_ptr().add(i));
                    let diff = _mm256_sub_ps(va, vb);
                    let sq = _mm256_mul_ps(diff, diff);
                    acc = _mm256_add_ps(acc, sq);
                    i += 8;
                }
                let hi = _mm256_extractf128_ps(acc, 1);
                let lo = _mm256_castps256_ps128(acc);
                let sum128 = _mm_add_ps(hi, lo);
                let shuf = _mm_movehdup_ps(sum128);
                let sums = _mm_add_ps(sum128, shuf);
                let shuf = _mm_movehl_ps(shuf, sums);
                let sums = _mm_add_ss(sums, shuf);
                sum += _mm_cvtss_f32(sums);
            }
        }
    }

    for j in i..len {
        let d = a[j] - b[j];
        sum += d * d;
    }
    (sum as f64).sqrt()
}

pub fn cosine(a: &[f32], b: &[f32]) -> f64 {
    let len = a.len().min(b.len());
    let mut dot = 0.0f32;
    let mut na = 0.0f32;
    let mut nb = 0.0f32;
    let mut i = 0;

    #[cfg(target_arch = "x86_64")]
    {
        if is_x86_feature_detected!("avx") && len >= 8 {
            unsafe {
                let mut acc_dot = _mm256_setzero_ps();
                let mut acc_na = _mm256_setzero_ps();
                let mut acc_nb = _mm256_setzero_ps();
                while i + 8 <= len {
                    let va = _mm256_loadu_ps(a.as_ptr().add(i));
                    let vb = _mm256_loadu_ps(b.as_ptr().add(i));
                    acc_dot = _mm256_add_ps(acc_dot, _mm256_mul_ps(va, vb));
                    acc_na = _mm256_add_ps(acc_na, _mm256_mul_ps(va, va));
                    acc_nb = _mm256_add_ps(acc_nb, _mm256_mul_ps(vb, vb));
                    i += 8;
                }
                dot += hsum256_ps(acc_dot);
                na += hsum256_ps(acc_na);
                nb += hsum256_ps(acc_nb);
            }
        }
    }

    for j in i..len {
        dot += a[j] * b[j];
        na += a[j] * a[j];
        nb += b[j] * b[j];
    }
    let denom = (na as f64).sqrt() * (nb as f64).sqrt();
    if denom < f64::EPSILON { return 1.0; }
    1.0 - (dot as f64) / denom
}

pub fn dot_product_distance(a: &[f32], b: &[f32]) -> f64 {
    let len = a.len().min(b.len());
    let mut sum = 0.0f32;
    let mut i = 0;

    #[cfg(target_arch = "x86_64")]
    {
        if is_x86_feature_detected!("avx") && len >= 8 {
            unsafe {
                let mut acc = _mm256_setzero_ps();
                while i + 8 <= len {
                    let va = _mm256_loadu_ps(a.as_ptr().add(i));
                    let vb = _mm256_loadu_ps(b.as_ptr().add(i));
                    acc = _mm256_add_ps(acc, _mm256_mul_ps(va, vb));
                    i += 8;
                }
                sum += hsum256_ps(acc);
            }
        }
    }

    for j in i..len {
        sum += a[j] * b[j];
    }
    -(sum as f64)
}

#[cfg(target_arch = "x86_64")]
#[inline]
unsafe fn hsum256_ps(v: __m256) -> f32 {
    let hi = _mm256_extractf128_ps(v, 1);
    let lo = _mm256_castps256_ps128(v);
    let sum128 = _mm_add_ps(hi, lo);
    let shuf = _mm_movehdup_ps(sum128);
    let sums = _mm_add_ps(sum128, shuf);
    let shuf = _mm_movehl_ps(shuf, sums);
    let sums = _mm_add_ss(sums, shuf);
    _mm_cvtss_f32(sums)
}
