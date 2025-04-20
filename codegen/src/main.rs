mod algebra;
mod ast;
mod compile;
mod emit;
mod glsl;
mod rust;

use crate::{
    algebra::{BasisElement, GeometricAlgebra, Involution, MultiVectorClass, MultiVectorClassRegistry, Product},
    ast::{AstNode, DataType, Parameter},
    emit::Emitter,
};

fn main() {
    // Parse command-line arguments
    let mut args = std::env::args();
    let _executable = args.next().unwrap(); // Discard executable name
    let config = args.next().unwrap(); // Get config string argument

    // Split config string by `;` into algebra and multivector class descriptors
    let mut config_iter = config.split(';');

    // Parse algebra descriptor: `name:squares`
    let algebra_descriptor = config_iter.next().unwrap();
    let mut algebra_descriptor_iter = algebra_descriptor.split(':');
    let algebra_name = algebra_descriptor_iter.next().unwrap();

    // Parse comma-separated list of generator squares into `Vec<isize>`
    let generator_squares = algebra_descriptor_iter
        .next()
        .unwrap()
        .split(',')
        .map(|x| x.parse::<isize>().unwrap())
        .collect::<Vec<_>>();

    // Create the algebra from the generator squares
    let algebra = GeometricAlgebra {
        generator_squares: generator_squares.as_slice(),
    };

    // Precompute involutions and products for the given algebra
    let involutions = Involution::involutions(&algebra);
    let products = Product::products(&algebra);

    // Print the Cayley table
    let basis = algebra.sorted_basis();
    for b in basis.iter() {
        for a in basis.iter() {
            print!("{:1$} ", BasisElement::product(a, b, &algebra), generator_squares.len() + 2);
        }
        println!();
    }

    // Register the user-defined multivector classes
    let mut registry = MultiVectorClassRegistry::default();
    for multi_vector_descriptor in config_iter {
        let mut multi_vector_descriptor_iter = multi_vector_descriptor.split(':');
        registry.register(MultiVectorClass {
            class_name: multi_vector_descriptor_iter.next().unwrap().to_owned(),
            grouped_basis: multi_vector_descriptor_iter
                .next()
                .unwrap()
                .split('|')
                .map(|group_descriptor| {
                    group_descriptor
                        .split(',')
                        .map(|element_name| BasisElement::parse(element_name, &algebra))
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>(),
        });
    }

    // Prepare output emitter for generated code in `../src/<algebra_name>/`
    let mut emitter = Emitter::new(&std::path::Path::new("../src/").join(std::path::Path::new(algebra_name)));

    // Emit preamble (e.g., `use`, common functions)
    emitter.emit(&AstNode::Preamble).unwrap();

    // Emit class definitions for all registered multivector types
    for class in registry.classes.iter() {
        emitter.emit(&AstNode::ClassDefinition { class }).unwrap();
    }

    // Emit trait impls
    let mut trait_implementations = std::collections::BTreeMap::new();
    for class_a in registry.classes.iter() {
        let parameter_a = Parameter {
            name: "self",
            data_type: DataType::MultiVector(class_a),
        };
        let mut single_trait_implementations = std::collections::BTreeMap::new();

        // Emit constants like Zero and One
        for name in &["Zero", "One"] {
            let ast_node = class_a.constant(name);
            emitter.emit(&ast_node).unwrap();
            if ast_node != AstNode::None {
                single_trait_implementations.insert(name.to_string(), ast_node);
            }
        }

        // Emit involutions (like Reverse, GradeInvolution)
        for (name, involution) in involutions.iter() {
            let ast_node = MultiVectorClass::involution(name, involution, &parameter_a, &registry, false);
            emitter.emit(&ast_node).unwrap();
            if ast_node != AstNode::None {
                single_trait_implementations.insert(name.to_string(), ast_node);
            }
        }

        // Emit pair trait impls
        let mut pair_trait_implementations = std::collections::BTreeMap::new();
        for class_b in registry.classes.iter() {
            let mut trait_implementations = std::collections::BTreeMap::new();
            let parameter_b = Parameter {
                name: "other",
                data_type: DataType::MultiVector(class_b),
            };

            // Emit conversions between different mv classes
            if class_a != class_b {
                let name = "Into";
                let ast_node = MultiVectorClass::involution(name, &Involution::projection(class_b), &parameter_a, &registry, true);
                emitter.emit(&ast_node).unwrap();
                if ast_node != AstNode::None {
                    trait_implementations.insert(name.to_string(), ast_node);
                }
            }

            // Emit element-wise Add/Sub
            for name in &["Add", "Sub"] {
                let ast_node = MultiVectorClass::element_wise(*name, &parameter_a, &parameter_b, &registry);
                emitter.emit(&ast_node).unwrap();
                if ast_node != AstNode::None {
                    trait_implementations.insert(name.to_string(), ast_node);
                }
            }

            // Emit element-wise Mul/Div (only for same class combinations)
            if class_a == class_b {
                for name in &["Mul", "Div"] {
                    let ast_node = MultiVectorClass::element_wise(*name, &parameter_a, &parameter_b, &registry);
                    emitter.emit(&ast_node).unwrap();
                    if ast_node != AstNode::None {
                        trait_implementations.insert(name.to_string(), ast_node);
                    }
                }
            }

            // Emit all defined algebraic products (geometric, inner, outer, etc.)
            for (name, product) in products.iter() {
                let ast_node = MultiVectorClass::product(name, product, &parameter_a, &parameter_b, &registry);
                emitter.emit(&ast_node).unwrap();
                if ast_node != AstNode::None {
                    trait_implementations.insert(name.to_string(), ast_node);
                }
            }

            // Store pair trait impls
            pair_trait_implementations.insert(
                parameter_b.multi_vector_class().class_name.clone(),
                (parameter_b.clone(), trait_implementations),
            );
        }

        // Derive magnitude and squared magnitude if scalar product and reversal exist
        for (parameter_b, pair_trait_implementations) in pair_trait_implementations.values() {
            if let Some(scalar_product) = pair_trait_implementations.get("ScalarProduct") {
                if let Some(reversal) = single_trait_implementations.get("Reversal") {
                    if parameter_a.multi_vector_class() == parameter_b.multi_vector_class() {
                        let squared_magnitude =
                            MultiVectorClass::derive_squared_magnitude("SquaredMagnitude", scalar_product, reversal, &parameter_a);
                        emitter.emit(&squared_magnitude).unwrap();

                        let magnitude = MultiVectorClass::derive_magnitude("Magnitude", &squared_magnitude, &parameter_a);
                        emitter.emit(&magnitude).unwrap();

                        single_trait_implementations.insert(result_of_trait!(squared_magnitude).name.to_string(), squared_magnitude);
                        single_trait_implementations.insert(result_of_trait!(magnitude).name.to_string(), magnitude);
                    }
                }
            }
        }

        // Derive scale, signum, inverse if applicable for scalar * multivector
        for (parameter_b, pair_trait_implementations) in pair_trait_implementations.values() {
            if let Some(geometric_product) = pair_trait_implementations.get("GeometricProduct") {
                if parameter_b.data_type.is_scalar() {
                    if !parameter_a.data_type.is_scalar() {
                        let scale = MultiVectorClass::derive_scale("Mul", geometric_product, &parameter_a, parameter_b);
                        emitter.emit(&scale).unwrap();
                    }
                    if let Some(magnitude) = single_trait_implementations.get("Magnitude") {
                        let signum = MultiVectorClass::derive_signum("Signum", geometric_product, magnitude, &parameter_a);
                        emitter.emit(&signum).unwrap();
                        single_trait_implementations.insert(result_of_trait!(signum).name.to_string(), signum);
                    }
                    if let Some(squared_magnitude) = single_trait_implementations.get("SquaredMagnitude") {
                        if let Some(reversal) = single_trait_implementations.get("Reversal") {
                            let inverse = MultiVectorClass::derive_inverse("Inverse", geometric_product, squared_magnitude, reversal, &parameter_a);
                            emitter.emit(&inverse).unwrap();
                            single_trait_implementations.insert(result_of_trait!(inverse).name.to_string(), inverse);
                        }
                    }
                }
            }
        }

        // Store all single and pair trait impls for class A
        trait_implementations.insert(
            parameter_a.multi_vector_class().class_name.clone(),
            (parameter_a.clone(), single_trait_implementations, pair_trait_implementations),
        );
    }

    // Final pass: derive higher-level traits like powi, quotient, transformation
    for (parameter_a, single_trait_implementations, pair_trait_implementations) in trait_implementations.values() {
        for (parameter_b, pair_trait_implementations) in pair_trait_implementations.values() {
            if let Some(geometric_product) = pair_trait_implementations.get("GeometricProduct") {
                let geometric_product_result = result_of_trait!(geometric_product);

                // Derive Powi when all types are the same
                if parameter_a.multi_vector_class() == parameter_b.multi_vector_class()
                    && geometric_product_result.multi_vector_class() == parameter_a.multi_vector_class()
                {
                    if let Some(constant_one) = single_trait_implementations.get("One") {
                        if let Some(inverse) = single_trait_implementations.get("Inverse") {
                            let power_of_integer = MultiVectorClass::derive_power_of_integer(
                                "Powi",
                                geometric_product,
                                constant_one,
                                inverse,
                                parameter_a,
                                &Parameter {
                                    name: "exponent",
                                    data_type: DataType::Integer,
                                },
                            );
                            emitter.emit(&power_of_integer).unwrap();
                        }
                    }
                }

                // Derive GeometricQuotient = a * b⁻¹
                if let Some(b_trait_implementations) = trait_implementations.get(&parameter_b.multi_vector_class().class_name) {
                    if let Some(inverse) = b_trait_implementations.1.get("Inverse") {
                        let division = MultiVectorClass::derive_division("GeometricQuotient", geometric_product, inverse, parameter_a, parameter_b);
                        emitter.emit(&division).unwrap();
                    }
                }

                // Derive Transformation = a * b * a⁻¹
                if let Some(reversal) = single_trait_implementations.get("Reversal") {
                    if let Some(b_trait_implementations) = trait_implementations.get(&geometric_product_result.multi_vector_class().class_name) {
                        if let Some(b_pair_trait_implementations) = b_trait_implementations.2.get(&parameter_a.multi_vector_class().class_name) {
                            if let Some(geometric_product_2) = b_pair_trait_implementations.1.get("GeometricProduct") {
                                let geometric_product_2_result = result_of_trait!(geometric_product_2);
                                if let Some(c_trait_implementations) =
                                    trait_implementations.get(&geometric_product_2_result.multi_vector_class().class_name)
                                {
                                    if let Some(c_pair_trait_implementations) =
                                        c_trait_implementations.2.get(&parameter_b.multi_vector_class().class_name)
                                    {
                                        let transformation = MultiVectorClass::derive_sandwich_product(
                                            "Transformation",
                                            geometric_product,
                                            geometric_product_2,
                                            reversal,
                                            c_pair_trait_implementations.1.get("Into"),
                                            parameter_a,
                                            parameter_b,
                                        );
                                        emitter.emit(&transformation).unwrap();
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
