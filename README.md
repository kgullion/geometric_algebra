[![actions](https://github.com/Lichtso/geometric_algebra/actions/workflows/actions.yaml/badge.svg)](https://github.com/Lichtso/geometric_algebra/actions/workflows/actions.yaml)
[![Docs](https://docs.rs/geometric_algebra/badge.svg)](https://docs.rs/geometric_algebra/)
[![crates.io](https://img.shields.io/crates/v/geometric_algebra.svg)](https://crates.io/crates/geometric_algebra)

## About
This repository allows you to describe [geometric algebras](https://en.wikipedia.org/wiki/Geometric_algebra) with 1 to 16 generator elements and generate SIMD-ready, dependency-less libraries for them. It also comes with a set of prebuilt projective geometric algebras in 1D, 2D and 3D which are elliptic, parabolic (euclidian) or hyperbolic.

## Architecture
- [DSL](https://en.wikipedia.org/wiki/Domain-specific_language) Parser: See [examples](.github/workflows/actions.yaml)
- Algebra: Generates the multiplication tables
- Compiler: Constructs an AST from the multiplication tables
- Optimizer: Simplifies the AST
- Legalizer: Inserts missing expressions in the AST
- Emitter: Serializes the AST to source code
    - [Rust](https://www.rust-lang.org/)
    - [GLSL](https://www.khronos.org/opengl/wiki/Core_Language_(GLSL))

## Supported SIMD ISAs
- x86, x86_64: sse2
- arm, aarch64: neon
- wasm32: simd128

## Usage

### Generating Code for a Custom Algebra

The code generator takes a descriptor string as input, which defines both the algebra and the multivector classes. The descriptor string format is:

```
algebra_name:squares;Class1:components;Class2:components...
```

Where:
- `algebra_name`: Name for your algebra
- `squares`: Comma-separated list of generator squares (1 for positive, 0 for null, -1 for negative)
- `Class1, Class2, ...`: Multivector classes you want to define
- `components`: Comma-separated list of basis elements for each class

#### Example Descriptor Strings

1. **Complex Numbers (EPGA1D)**: 
   ```
   epga1d:1,1;Scalar:1;ComplexNumber:1,e01
   ```

2. **Parabolic Projective Geometric Algebra in 3D**:
   ```
   ppga3d:0,1,1,1;Scalar:1;Rotor:1,e23,-e13,e12;Point:e123,-e023,e013,-e012
   ```

### Using the Code Generator

1. **Build the code generator**:
   ```bash
   cargo build --manifest-path codegen/Cargo.toml
   ```

2. **Run the code generator with your descriptor**:
   ```bash
   cd codegen
   ./target/debug/codegen "algebra_name:squares;Class1:components;Class2:components..."
   ```

3. **Generated code** will be output to `src/<algebra_name>/` as Rust and GLSL implementations.

### Using with Rust

After generating your algebra code:

```rust
// Import your custom algebra
use geometric_algebra::algebra_name::{Class1, Class2};

// Create multivector instances
let mv1 = Class1::new(...);
let mv2 = Class2::new(...);

// Use algebraic operations
let product = mv1 * mv2;  // Geometric product
let sum = mv1 + mv2;      // Addition (when types match)
```

### Available Prebuilt Algebras

The library includes several prebuilt algebras:

- **1D**:
  - `epga1d`: Elliptic (Complex Numbers)
  - `ppga1d`: Parabolic (Dual Numbers)
  - `hpga1d`: Hyperbolic (Split Complex Numbers)

- **2D**:
  - `epga2d`: Elliptic
  - `ppga2d`: Parabolic (Euclidean)
  - `hpga2d`: Hyperbolic

- **3D**:
  - `epga3d`: Elliptic
  - `ppga3d`: Parabolic (Euclidean)
  - `hpga3d`: Hyperbolic

Each algebra comes with predefined multivector classes like `Scalar`, `Rotor`, `Point`, `Line`, `Plane`, `Motor`, etc.

### Example Usage

```rust
use geometric_algebra::ppga3d::Point;

fn main() {
  let point = Point::new(1.23, 0.23, 0.13, 0.12);
  println!("{:?}", point)
}
```
