/// Represents a geometric algebra, as defined by the squares of its generators
pub struct GeometricAlgebra<'a> {
    pub generator_squares: &'a [isize],
}

impl<'a> GeometricAlgebra<'a> {
    /// Number of basis blades in the algebra (2^n)
    pub fn basis_size(&self) -> usize {
        1 << self.generator_squares.len()
    }

    /// Iterator over all basis blades
    /// Uses duals to normalize the scalar values for canonical ordering.
    pub fn basis(&self) -> impl Iterator<Item = BasisElement> + '_ {
        (0..self.basis_size() as BasisElementIndex).map(move |index| {
            let mut element = BasisElement::from_index(index);
            let dual = element.dual(self);
            if dual.cmp(&element) == std::cmp::Ordering::Less {
                element.scalar = element.dual(self).scalar;
            }
            element
        })
    }

    /// Sorted list of all basis blades, in canonical order.
    pub fn sorted_basis(&self) -> Vec<BasisElement> {
        let mut basis_elements = self.basis().collect::<Vec<BasisElement>>();
        basis_elements.sort();
        basis_elements
    }
}

/// Index type for basis blades (supports up to 16 generators).
pub type BasisElementIndex = u16;

/// Represents a single basis blade with a scalar and an index (bitmask).
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct BasisElement {
    pub scalar: isize,
    pub index: BasisElementIndex,
}

impl BasisElement {
    /// Constructs a basis element from its index with scalar 1
    pub fn from_index(index: BasisElementIndex) -> Self {
        Self { scalar: 1, index }
    }

    /// Parses a basis blade from a string like "-e13"
    pub fn parse(mut name: &str, algebra: &GeometricAlgebra) -> Self {
        let mut result = Self::from_index(0);
        if name.starts_with('-') {
            name = &name[1..];
            result.scalar = -1;
        }
        if name == "1" {
            return result;
        }
        let mut generator_indices = name.chars();
        assert_eq!(generator_indices.next().unwrap(), 'e');
        for generator_index in generator_indices {
            let generator_index = generator_index.to_digit(16).unwrap();
            assert!((generator_index as usize) < algebra.generator_squares.len());
            result = BasisElement::product(&result, &Self::from_index(1 << generator_index), algebra);
        }
        result
    }

    /// Number of basis vectors in the element
    pub fn grade(&self) -> usize {
        self.index.count_ones() as usize
    }

    /// Iterator over the bit indices of the basis vectors present in the element
    pub fn component_bits(&self) -> impl Iterator<Item = usize> + '_ {
        (0..std::mem::size_of::<BasisElementIndex>() * 8).filter(move |index| (self.index >> index) & 1 != 0)
    }

    /// Dual of the element
    pub fn dual(&self, algebra: &GeometricAlgebra) -> Self {
        let mut result = Self {
            scalar: self.scalar,
            index: algebra.basis_size() as BasisElementIndex - 1 - self.index,
        };
        result.scalar *= BasisElement::product(self, &result, algebra).scalar;
        result
    }

    /// Geometric product
    pub fn product(a: &Self, b: &Self, algebra: &GeometricAlgebra) -> Self {
        let commutations = a.component_bits().fold((0, a.index, b.index), |(commutations, a, b), index| {
            let hurdles_a = a & (BasisElementIndex::MAX << (index + 1));
            let hurdles_b = b & ((1 << index) - 1);
            (
                commutations + Self::from_index(hurdles_a | hurdles_b).grade(),
                a & !(1 << index),
                b ^ (1 << index),
            )
        });
        Self {
            scalar: Self::from_index(a.index & b.index)
                .component_bits()
                .map(|i| algebra.generator_squares[i])
                .fold(a.scalar * b.scalar * if commutations.0 % 2 == 0 { 1 } else { -1 }, |a, b| a * b),
            index: a.index ^ b.index,
        }
    }
}

impl std::fmt::Display for BasisElement {
    /// Custom string representation for basis blades like "e12" or "-e1".
    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        let name = format!("e{}", self.component_bits().map(|index| format!("{:X}", index)).collect::<String>());
        formatter.pad_integral(
            self.scalar >= 0,
            "",
            if self.scalar == 0 {
                "0"
            } else if self.index == 0 {
                "1"
            } else {
                name.as_str()
            },
        )
    }
}

impl std::cmp::Ord for BasisElement {
    /// Basis elements are ordered by grade, then lexicographically by index.
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        let grades_order = self.grade().cmp(&other.grade());
        if grades_order != std::cmp::Ordering::Equal {
            return grades_order;
        }
        let a_without_b = self.index & (!other.index);
        let b_without_a = other.index & (!self.index);
        if a_without_b.trailing_zeros() < b_without_a.trailing_zeros() {
            std::cmp::Ordering::Less
        } else {
            std::cmp::Ordering::Greater
        }
    }
}

impl std::cmp::PartialOrd for BasisElement {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

/// Basis involutions (e.g. projection, involutions, duals etc)
#[derive(Clone)]
pub struct Involution {
    pub terms: Vec<(BasisElement, BasisElement)>,
}

impl Involution {
    /// Identity involution: each basis maps to itself.
    pub fn identity(algebra: &GeometricAlgebra) -> Self {
        Self {
            terms: algebra.basis().map(|element| (element.clone(), element)).collect(),
        }
    }

    /// Projection that retains only elements in the given multivector class.
    pub fn projection(class: &MultiVectorClass) -> Self {
        Self {
            terms: class.flat_basis().iter().map(|element| (element.clone(), element.clone())).collect(),
        }
    }

    /// Applies a negation to selected grades.
    pub fn negated<F>(&self, grade_negation: F) -> Self
    where
        F: Fn(usize) -> bool,
    {
        Self {
            terms: self
                .terms
                .iter()
                .map(|(key, value)| {
                    let mut element = value.clone();
                    element.scalar *= if grade_negation(value.grade()) { -1 } else { 1 };
                    (key.clone(), element)
                })
                .collect(),
        }
    }

    /// Applies dual to all target values in the involution.
    pub fn dual(&self, algebra: &GeometricAlgebra) -> Self {
        Self {
            terms: self.terms.iter().map(|(key, value)| (key.clone(), value.dual(algebra))).collect(),
        }
    }

    /// Predefined involutions for geometric algebra.
    pub fn involutions(algebra: &GeometricAlgebra) -> Vec<(&'static str, Self)> {
        let involution = Self::identity(algebra);
        vec![
            ("Neg", involution.negated(|_grade| true)),
            ("Automorphism", involution.negated(|grade| grade % 2 == 1)),
            ("Reversal", involution.negated(|grade| grade % 4 >= 2)),
            ("Conjugation", involution.negated(|grade| (grade + 3) % 4 < 2)),
            ("Dual", involution.dual(algebra)),
        ]
    }
}

/// A single product term with explicit factors and result.
#[derive(Clone, PartialEq, Eq)]
pub struct ProductTerm {
    pub product: BasisElement,
    pub factor_a: BasisElement,
    pub factor_b: BasisElement,
}

/// Represents the full product table between two multivectors.
#[derive(Clone)]
pub struct Product {
    pub terms: Vec<ProductTerm>,
}

impl Product {
    /// Constructs the full bilinear product between all elements of `a` and `b`.
    pub fn new(a: &[BasisElement], b: &[BasisElement], algebra: &GeometricAlgebra) -> Self {
        Self {
            terms: a
                .iter()
                .flat_map(|a| {
                    b.iter().map(move |b| ProductTerm {
                        product: BasisElement::product(a, b, algebra),
                        factor_a: a.clone(),
                        factor_b: b.clone(),
                    })
                })
                .filter(|term| term.product.scalar != 0)
                .collect(),
        }
    }

    /// Filters product terms based on a projection function of grades.
    pub fn projected<F>(&self, grade_projection: F) -> Self
    where
        F: Fn(usize, usize, usize) -> bool,
    {
        Self {
            terms: self
                .terms
                .iter()
                .filter(|term| grade_projection(term.factor_a.grade(), term.factor_b.grade(), term.product.grade()))
                .cloned()
                .collect(),
        }
    }

    /// Dualizes each term in the product.
    pub fn dual(&self, algebra: &GeometricAlgebra) -> Self {
        Self {
            terms: self
                .terms
                .iter()
                .map(|term| ProductTerm {
                    product: term.product.dual(algebra),
                    factor_a: term.factor_a.dual(algebra),
                    factor_b: term.factor_b.dual(algebra),
                })
                .collect(),
        }
    }

    /// Computes all standard products from the full geometric product.
    pub fn products(algebra: &GeometricAlgebra) -> Vec<(&'static str, Self)> {
        let basis = algebra.basis().collect::<Vec<_>>();
        let product = Self::new(&basis, &basis, algebra);
        vec![
            ("GeometricProduct", product.clone()),
            ("RegressiveProduct", product.projected(|r, s, t| t == r + s).dual(algebra)),
            ("OuterProduct", product.projected(|r, s, t| t == r + s)),
            ("InnerProduct", product.projected(|r, s, t| t == (r as isize - s as isize).unsigned_abs())),
            ("LeftContraction", product.projected(|r, s, t| t as isize == s as isize - r as isize)),
            ("RightContraction", product.projected(|r, s, t| t as isize == r as isize - s as isize)),
            ("ScalarProduct", product.projected(|_r, _s, t| t == 0)),
        ]
    }
}

/// Registry to deduplicate and retrieve multivector classes by signature.
#[derive(Default)]
pub struct MultiVectorClassRegistry {
    pub classes: Vec<MultiVectorClass>,
    index_by_signature: std::collections::HashMap<Vec<BasisElementIndex>, usize>,
}

impl MultiVectorClassRegistry {
    /// Add class to registry
    pub fn register(&mut self, class: MultiVectorClass) {
        self.index_by_signature.insert(class.signature(), self.classes.len());
        self.classes.push(class);
    }

    /// Get class by signature
    pub fn get(&self, signature: &[BasisElementIndex]) -> Option<&MultiVectorClass> {
        self.index_by_signature.get(signature).map(|index| &self.classes[*index])
    }
}

/// A named collection of basis elements (e.g. even subalgebra, scalar, bivector, rotor etc)
#[derive(PartialEq, Eq, Debug)]
pub struct MultiVectorClass {
    pub class_name: String,
    pub grouped_basis: Vec<Vec<BasisElement>>,
}
