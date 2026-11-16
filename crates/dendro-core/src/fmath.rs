//! Fonctions transcendantes déterministes.
//!
//! `f32::sin`, `f64::ln`, `powf`... appellent la libm de la plateforme (glibc, UCRT, compiler-rt) :
//! précises à ~1 ulp mais pas identiques bit à bit d'un OS à l'autre. Ici tout passe par la crate
//! `libm` (Rust pur) : mêmes résultats partout. Les opérations exactes IEEE (`+ - * /`, `sqrt`,
//! `abs`, `min`, `max`, `floor`, `ceil`, `round`, `clamp`...) restent les méthodes de `std`.
//!
//! Règle : dans `dendro-core/src`, jamais de `.sin()`, `.cos()`, `.exp()`, `.powf()`, `.powi()`...
//! bruts ; utiliser `.dsin()`, `.dcos()`, `.dexp()`, `.dpowf()`, `.dpowi()`... (garde-fou :
//! `tests/determinism.rs`).

/// Méthodes flottantes déterministes (préfixe `d`).
pub trait DMath: Sized {
    fn dsin(self) -> Self;
    fn dcos(self) -> Self;
    fn dtan(self) -> Self;
    fn dacos(self) -> Self;
    fn datan2(self, x: Self) -> Self;
    fn dexp(self) -> Self;
    fn dln(self) -> Self;
    fn dpowf(self, y: Self) -> Self;
    /// Puissance entière par multiplications dans un ordre fixe (exponentiation par carrés).
    fn dpowi(self, n: i32) -> Self;
    fn dsin_cos(self) -> (Self, Self);
}

macro_rules! impl_dmath {
    ($t:ty, $sin:ident, $cos:ident, $tan:ident, $acos:ident, $atan2:ident, $exp:ident, $ln:ident, $pow:ident) => {
        impl DMath for $t {
            #[inline]
            fn dsin(self) -> Self {
                libm::$sin(self)
            }
            #[inline]
            fn dcos(self) -> Self {
                libm::$cos(self)
            }
            #[inline]
            fn dtan(self) -> Self {
                libm::$tan(self)
            }
            #[inline]
            fn dacos(self) -> Self {
                libm::$acos(self)
            }
            #[inline]
            fn datan2(self, x: Self) -> Self {
                libm::$atan2(self, x)
            }
            #[inline]
            fn dexp(self) -> Self {
                libm::$exp(self)
            }
            #[inline]
            fn dln(self) -> Self {
                libm::$ln(self)
            }
            #[inline]
            fn dpowf(self, y: Self) -> Self {
                libm::$pow(self, y)
            }
            #[inline]
            fn dpowi(self, n: i32) -> Self {
                let mut e = n.unsigned_abs();
                let mut base = self;
                let mut acc: $t = 1.0;
                while e > 0 {
                    if e & 1 == 1 {
                        acc *= base;
                    }
                    e >>= 1;
                    if e > 0 {
                        base *= base;
                    }
                }
                if n < 0 {
                    1.0 / acc
                } else {
                    acc
                }
            }
            #[inline]
            fn dsin_cos(self) -> (Self, Self) {
                (libm::$sin(self), libm::$cos(self))
            }
        }
    };
}

impl_dmath!(f32, sinf, cosf, tanf, acosf, atan2f, expf, logf, powf);
impl_dmath!(f64, sin, cos, tan, acos, atan2, exp, log, pow);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn powi_matches_repeated_multiplication() {
        assert_eq!(3.0f32.dpowi(2), 9.0);
        assert_eq!(2.0f32.dpowi(-2), 0.25);
        assert_eq!(0.75f32.dpowi(0), 1.0);
        assert_eq!(2.0f64.dpowi(10), 1024.0);
    }

    #[test]
    fn close_to_std() {
        for i in 0..100 {
            let x = i as f32 * 0.07 - 3.0;
            assert!((x.dsin() - x.sin()).abs() < 1e-6);
            assert!((x.dcos() - x.cos()).abs() < 1e-6);
            assert!((x.dexp() - x.exp()).abs() <= 1e-6 * x.exp());
            assert!((x.abs().dpowf(1.3) - x.abs().powf(1.3)).abs() < 1e-5);
        }
    }
}
