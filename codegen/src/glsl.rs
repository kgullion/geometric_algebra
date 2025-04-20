/// GLSL code generation from AST
use crate::{
    ast::{AstNode, DataType, Expression, ExpressionContent},
    emit::{camel_to_snake_case, emit_indentation},
};

/// Component names for GLSL vector swizzling
const COMPONENT: &[&str] = &["x", "y", "z", "w"];

/// Emits the GLSL representation of a data type
fn emit_data_type<W: std::io::Write>(collector: &mut W, data_type: &DataType) -> std::io::Result<()> {
    match data_type {
        DataType::Integer => collector.write_all(b"int"),
        DataType::SimdVector(size) if *size == 1 => collector.write_all(b"float"), // Size-1 vectors are floats
        DataType::SimdVector(size) => collector.write_fmt(format_args!("vec{}", *size)), // vecN notation
        DataType::MultiVector(class) if class.is_scalar() => collector.write_all(b"float"), // Scalar multivectors
        DataType::MultiVector(class) => collector.write_all(class.class_name.as_bytes()), // Other multivectors
    }
}

/// Recursively emits GLSL code for an expression
fn emit_expression<W: std::io::Write>(collector: &mut W, expression: &Expression) -> std::io::Result<()> {
    match &expression.content {
        ExpressionContent::None => unreachable!(),

        // Variable reference
        ExpressionContent::Variable(_data_type, name) => {
            collector.write_all(name.bytes().collect::<Vec<_>>().as_slice())?;
        }

        // Special case for scalar constructor calls
        ExpressionContent::InvokeClassMethod(class, "Constructor", arguments) if class.is_scalar() => {
            emit_expression(collector, &arguments[0].1)?;
        }

        // Method invocations (both class and instance methods)
        ExpressionContent::InvokeClassMethod(_, _, arguments) | ExpressionContent::InvokeInstanceMethod(_, _, _, _, arguments) => {
            match &expression.content {
                // Instance method call
                ExpressionContent::InvokeInstanceMethod(result_class, inner_expression, method_name, _, _) => {
                    // Build function name from result class, argument classes, and method name
                    if let DataType::MultiVector(result_class) = result_class {
                        camel_to_snake_case(collector, &result_class.class_name)?;
                        collector.write_all(b"_")?;
                    }
                    // Include argument class names in function name
                    for (argument_class, _argument) in arguments.iter() {
                        if let DataType::MultiVector(argument_class) = argument_class {
                            camel_to_snake_case(collector, &argument_class.class_name)?;
                            collector.write_all(b"_")?;
                        }
                    }
                    camel_to_snake_case(collector, method_name)?;

                    // Start function call and emit instance expression
                    collector.write_all(b"(")?;
                    emit_expression(collector, inner_expression)?;
                    if !arguments.is_empty() {
                        collector.write_all(b", ")?;
                    }
                }

                // Class method call
                ExpressionContent::InvokeClassMethod(class, method_name, _) => {
                    if *method_name == "Constructor" {
                        // Constructor call
                        collector.write_fmt(format_args!("{}", &class.class_name))?;
                    } else {
                        // Static method call
                        camel_to_snake_case(collector, &class.class_name)?;
                        collector.write_all(b"_")?;
                        camel_to_snake_case(collector, method_name)?;
                    }
                    collector.write_all(b"(")?;
                }
                _ => unreachable!(),
            }

            // Emit all arguments, comma-separated
            for (i, (_argument_class, argument)) in arguments.iter().enumerate() {
                if i > 0 {
                    collector.write_all(b", ")?;
                }
                emit_expression(collector, argument)?;
            }
            collector.write_all(b")")?;
        }

        // Type conversion
        ExpressionContent::Conversion(source_class, destination_class, inner_expression) => {
            // Format: source_destination_into(expr)
            camel_to_snake_case(collector, &source_class.class_name)?;
            collector.write_all(b"_")?;
            camel_to_snake_case(collector, &destination_class.class_name)?;
            collector.write_all(b"_into(")?;
            emit_expression(collector, inner_expression)?;
            collector.write_all(b")")?;
        }

        // Ternary conditional operator
        ExpressionContent::Select(condition_expression, then_expression, else_expression) => {
            collector.write_all(b"(")?;
            emit_expression(collector, condition_expression)?;
            collector.write_all(b") ? ")?;
            emit_expression(collector, then_expression)?;
            collector.write_all(b" : ")?;
            emit_expression(collector, else_expression)?;
        }

        // Array/vector element access using GLSL's .gN syntax
        ExpressionContent::Access(inner_expression, array_index) => {
            emit_expression(collector, inner_expression)?;
            if !inner_expression.is_scalar() {
                collector.write_fmt(format_args!(".g{}", array_index))?;
            }
        }

        // GLSL vector swizzling (.xyzw)
        ExpressionContent::Swizzle(inner_expression, indices) => {
            emit_expression(collector, inner_expression)?;
            collector.write_all(b".")?;
            for component_index in indices.iter() {
                collector.write_all(COMPONENT[*component_index].bytes().collect::<Vec<_>>().as_slice())?;
            }
        }

        // Complex indexing for gathering components from potentially multiple vectors
        ExpressionContent::Gather(inner_expression, indices) => {
            if expression.size == 1 && inner_expression.is_scalar() {
                // Simple case - just emit the inner expression
                emit_expression(collector, inner_expression)?;
            } else {
                // Vector construction from components
                if expression.size > 1 {
                    emit_data_type(collector, &DataType::SimdVector(expression.size))?;
                    collector.write_all(b"(")?;
                }

                // Generate each component access
                for (i, (array_index, component_index)) in indices.iter().enumerate() {
                    if i > 0 {
                        collector.write_all(b", ")?;
                    }
                    emit_expression(collector, inner_expression)?;
                    if !inner_expression.is_scalar() {
                        // Access array element
                        collector.write_fmt(format_args!(".g{}", array_index))?;
                        if inner_expression.size > 1 {
                            // Access component within vector
                            collector.write_fmt(format_args!(".{}", COMPONENT[*component_index]))?;
                        }
                    }
                }

                if expression.size > 1 {
                    collector.write_all(b")")?;
                }
            }
        }

        // Constant value emission
        ExpressionContent::Constant(data_type, values) => match data_type {
            DataType::Integer => collector.write_fmt(format_args!("{}", values[0] as f32))?,
            DataType::SimdVector(_size) => {
                if expression.size == 1 {
                    // Scalar constant with decimal point
                    collector.write_fmt(format_args!("{:.1}", values[0] as f32))?
                } else {
                    // Vector constructor with components
                    emit_data_type(collector, &DataType::SimdVector(expression.size))?;
                    collector.write_fmt(format_args!(
                        "({})",
                        values.iter().map(|value| format!("{:.1}", *value as f32)).collect::<Vec<_>>().join(", ")
                    ))?
                }
            }
            _ => unreachable!(),
        },

        // Mathematical function
        ExpressionContent::SquareRoot(inner_expression) => {
            collector.write_all(b"sqrt(")?;
            emit_expression(collector, inner_expression)?;
            collector.write_all(b")")?;
        }

        // Binary operations
        ExpressionContent::Add(lhs, rhs)
        | ExpressionContent::Subtract(lhs, rhs)
        | ExpressionContent::Multiply(lhs, rhs)
        | ExpressionContent::Divide(lhs, rhs)
        | ExpressionContent::LessThan(lhs, rhs)
        | ExpressionContent::Equal(lhs, rhs)
        | ExpressionContent::LogicAnd(lhs, rhs)
        | ExpressionContent::BitShiftRight(lhs, rhs) => {
            // Add parentheses for logical AND to ensure correct precedence
            if let ExpressionContent::LogicAnd(_, _) = expression.content {
                collector.write_all(b"(")?;
            }

            // Left operand
            emit_expression(collector, lhs)?;

            // Operator
            collector.write_all(match expression.content {
                ExpressionContent::Add(_, _) => b" + ",
                ExpressionContent::Subtract(_, _) => b" - ",
                ExpressionContent::Multiply(_, _) => b" * ",
                ExpressionContent::Divide(_, _) => b" / ",
                ExpressionContent::LessThan(_, _) => b" < ",
                ExpressionContent::Equal(_, _) => b" == ",
                ExpressionContent::LogicAnd(_, _) => b" & ",
                ExpressionContent::BitShiftRight(_, _) => b" >> ",
                _ => unreachable!(),
            })?;

            // Right operand
            emit_expression(collector, rhs)?;
            if let ExpressionContent::LogicAnd(_, _) = expression.content {
                collector.write_all(b")")?;
            }
        }
    }
    Ok(())
}

/// Main function to emit GLSL code for an AST node
pub fn emit_code<W: std::io::Write>(collector: &mut W, ast_node: &AstNode, indentation: usize) -> std::io::Result<()> {
    match ast_node {
        AstNode::None => {}
        AstNode::Preamble => {}

        // Struct definition for multivector class
        AstNode::ClassDefinition { class } => {
            if class.is_scalar() {
                return Ok(()); // Skip scalar classes
            }

            // Generate struct with fields for basis element groups
            collector.write_fmt(format_args!("struct {} {{\n", class.class_name))?;
            for (i, group) in class.grouped_basis.iter().enumerate() {
                // Comment showing the basis elements in this group
                emit_indentation(collector, indentation + 1)?;
                collector.write_all(b"// ")?;
                for (i, element) in group.iter().enumerate() {
                    if i > 0 {
                        collector.write_all(b", ")?;
                    }
                    collector.write_fmt(format_args!("{}", element))?;
                }
                collector.write_all(b"\n")?;

                // Field declaration using appropriate vector type
                emit_indentation(collector, indentation + 1)?;
                emit_data_type(collector, &DataType::SimdVector(group.len()))?;
                collector.write_fmt(format_args!(" g{};\n", i))?;
            }
            emit_indentation(collector, indentation)?;
            collector.write_all(b"};\n\n")?;
        }

        // Return statement
        AstNode::ReturnStatement { expression } => {
            collector.write_all(b"return ")?;
            emit_expression(collector, expression)?;
            collector.write_all(b";\n")?;
        }

        // Variable declaration/assignment
        AstNode::VariableAssignment { name, data_type, expression } => {
            if let Some(data_type) = data_type {
                // Include type for declarations
                emit_data_type(collector, data_type)?;
                collector.write_all(b" ")?;
            }
            collector.write_fmt(format_args!("{} = ", name))?;
            emit_expression(collector, expression)?;
            collector.write_all(b";\n")?;
        }

        // If and while blocks
        AstNode::IfThenBlock { condition, body } | AstNode::WhileLoopBlock { condition, body } => {
            collector.write_all(match &ast_node {
                AstNode::IfThenBlock { .. } => b"if",
                AstNode::WhileLoopBlock { .. } => b"while",
                _ => unreachable!(),
            })?;

            // Condition and block opening
            collector.write_all(b"(")?;
            emit_expression(collector, condition)?;
            collector.write_all(b") {\n")?;

            // Body statements
            for statement in body.iter() {
                emit_indentation(collector, indentation + 1)?;
                emit_code(collector, statement, indentation + 1)?;
            }

            // Block closing
            emit_indentation(collector, indentation)?;
            collector.write_all(b"}\n")?;
        }

        // Function definition
        AstNode::TraitImplementation { result, parameters, body } => {
            // Return type and function name construction
            emit_data_type(collector, &result.data_type)?;
            collector.write_all(b" ")?;

            // Generate function name based on parameter count and types
            match parameters.len() {
                0 => camel_to_snake_case(collector, &result.multi_vector_class().class_name)?,
                1 if result.name == "Into" => {
                    // Special case for conversion functions
                    camel_to_snake_case(collector, &parameters[0].multi_vector_class().class_name)?;
                    collector.write_all(b"_")?;
                    camel_to_snake_case(collector, &result.multi_vector_class().class_name)?;
                }
                1 => camel_to_snake_case(collector, &parameters[0].multi_vector_class().class_name)?,
                2 if !matches!(parameters[1].data_type, DataType::MultiVector(_)) => {
                    // Method with one multivector and one non-multivector parameter
                    camel_to_snake_case(collector, &parameters[0].multi_vector_class().class_name)?
                }
                2 => {
                    // Method with two multivector parameters
                    camel_to_snake_case(collector, &parameters[0].multi_vector_class().class_name)?;
                    collector.write_all(b"_")?;
                    camel_to_snake_case(collector, &parameters[1].multi_vector_class().class_name)?;
                }
                _ => unreachable!(),
            }
            collector.write_all(b"_")?;
            camel_to_snake_case(collector, result.name)?;

            // Function parameters
            collector.write_all(b"(")?;
            for (i, parameter) in parameters.iter().enumerate() {
                if i > 0 {
                    collector.write_all(b", ")?;
                }
                emit_data_type(collector, &parameter.data_type)?;
                collector.write_fmt(format_args!(" {}", parameter.name))?;
            }
            collector.write_all(b") {\n")?;

            // Function body
            for statement in body.iter() {
                emit_indentation(collector, indentation + 1)?;
                emit_code(collector, statement, indentation + 1)?;
            }

            // Function closing
            emit_indentation(collector, indentation)?;
            collector.write_all(b"}\n\n")?;
        }
    }
    Ok(())
}
