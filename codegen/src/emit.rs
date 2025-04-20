/// Code emission utilities and main emitter implementation
/// This code bridges the abstract syntax tree (AST) representation
/// with concrete code generation, allowing the same DSL to target
/// both CPU-side Rust code and GPU-side GLSL shader code
use crate::{algebra::BasisElement, ast::AstNode, glsl, rust};

/// Converts camelCase names to snake_case (with handling of consecutive uppercase letters)
pub fn camel_to_snake_case<W: std::io::Write>(collector: &mut W, name: &str) -> std::io::Result<()> {
    // Find positions of uppercase characters to insert underscores
    let mut underscores = name.chars().enumerate().filter(|(_i, c)| c.is_uppercase()).map(|(i, _c)| i).peekable();

    // Process each character in the name
    for (i, c) in name.to_lowercase().bytes().enumerate() {
        if let Some(next_underscores) = underscores.peek() {
            if i == *next_underscores {
                // Insert underscore before uppercase letter (except at the beginning)
                if i > 0 {
                    collector.write_all(b"_")?;
                }
                underscores.next();
            }
        }
        collector.write_all(&[c])?; // Write the lowercase character
    }
    Ok(())
}

/// Emits spaces for code indentation
pub fn emit_indentation<W: std::io::Write>(collector: &mut W, indentation: usize) -> std::io::Result<()> {
    for _ in 0..indentation {
        collector.write_all(b"    ")?;
    }
    Ok(())
}

/// Emits the name for a basis element in geometric algebra
pub fn emit_element_name<W: std::io::Write>(collector: &mut W, element: &BasisElement) -> std::io::Result<()> {
    debug_assert_ne!(element.scalar, 0); // Verify element is non-zero

    if element.index == 0 {
        // Special case for scalar element
        collector.write_all(b"scalar")
    } else {
        // For other basis elements, use e notation with sign and indices
        collector.write_all(if element.scalar < 0 { b"_e" } else { b"e" })?;

        // Convert binary component bits to hexadecimal representation
        collector.write_all(
            element
                .component_bits()
                .map(|index| format!("{:X}", index))
                .collect::<String>()
                .as_bytes(),
        )
    }
}

/// Main code emitter (handles both Rust and GLSL output)
pub struct Emitter<W: std::io::Write> {
    pub rust_collector: W,
    pub glsl_collector: W,
}

impl Emitter<std::fs::File> {
    /// Output code to .rs and .glsl files
    pub fn new(path: &std::path::Path) -> Self {
        Self {
            // Create output files with appropriate extensions
            rust_collector: std::fs::File::create(path.with_extension("rs")).unwrap(),
            glsl_collector: std::fs::File::create(path.with_extension("glsl")).unwrap(),
        }
    }
}

impl<W: std::io::Write> Emitter<W> {
    /// Emits code for both Rust and GLSL from an AST node
    pub fn emit(&mut self, ast_node: &AstNode) -> std::io::Result<()> {
        rust::emit_code(&mut self.rust_collector, ast_node, 0)?;
        glsl::emit_code(&mut self.glsl_collector, ast_node, 0)?;
        Ok(())
    }
}
