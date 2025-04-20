/// AST node for data types
use crate::algebra::MultiVectorClass;

#[derive(PartialEq, Eq, Clone, Debug)]
pub enum DataType<'a> {
    Integer,                           // Basic integer type
    SimdVector(usize),                 // SIMD vector with specific size
    MultiVector(&'a MultiVectorClass), // Reference to a multi-vector class
}

impl DataType<'_> {
    /// Determines if the data type represents a scalar value
    pub fn is_scalar(&self) -> bool {
        match self {
            Self::SimdVector(1) => true,                                             // A size-1 SIMD vector is considered a scalar
            Self::MultiVector(multi_vector_class) => multi_vector_class.is_scalar(), // Delegate to the class
            _ => false,                                                              // Other types are not scalars
        }
    }
}

/// ExpressionContent represents the AST expression nodes
#[derive(PartialEq, Eq, Clone, Debug)]
pub enum ExpressionContent<'a> {
    None,
    Variable(DataType<'a>, &'static str), // Variable with type and name

    // Method invocation on a class
    InvokeClassMethod(&'a MultiVectorClass, &'static str, Vec<(DataType<'a>, Expression<'a>)>),

    // Method invocation on an instance
    InvokeInstanceMethod(
        DataType<'a>,                        // Return type
        Box<Expression<'a>>,                 // Instance expression
        &'static str,                        // Method name
        DataType<'a>,                        // Object type
        Vec<(DataType<'a>, Expression<'a>)>, // Arguments with types
    ),

    Conversion(&'a MultiVectorClass, &'a MultiVectorClass, Box<Expression<'a>>), // Type conversion
    Select(Box<Expression<'a>>, Box<Expression<'a>>, Box<Expression<'a>>),       // Ternary selection
    Access(Box<Expression<'a>>, usize),                                          // Array/vector access
    Swizzle(Box<Expression<'a>>, Vec<usize>),                                    // Component reordering
    Gather(Box<Expression<'a>>, Vec<(usize, usize)>),                            // Complex indexing
    Constant(DataType<'a>, Vec<isize>),                                          // Constant values

    // Mathematical operations
    SquareRoot(Box<Expression<'a>>),
    Add(Box<Expression<'a>>, Box<Expression<'a>>),
    Subtract(Box<Expression<'a>>, Box<Expression<'a>>),
    Multiply(Box<Expression<'a>>, Box<Expression<'a>>),
    Divide(Box<Expression<'a>>, Box<Expression<'a>>),

    // Comparison and logical operations
    LessThan(Box<Expression<'a>>, Box<Expression<'a>>),
    Equal(Box<Expression<'a>>, Box<Expression<'a>>),
    LogicAnd(Box<Expression<'a>>, Box<Expression<'a>>),
    BitShiftRight(Box<Expression<'a>>, Box<Expression<'a>>),
}

/// AST node for code blocks
#[derive(PartialEq, Eq, Clone, Debug)]
pub struct Expression<'a> {
    pub size: usize,                    // Size of the expression (vector length)
    pub content: ExpressionContent<'a>, // Actual expression content
}

impl Expression<'_> {
    /// Determines if an expression is scalar
    pub fn is_scalar(&self) -> bool {
        if self.size > 1 {
            return false; // Multi-element expressions are not scalars
        }
        match &self.content {
            ExpressionContent::Variable(data_type, _) => data_type.is_scalar(),
            ExpressionContent::InvokeInstanceMethod(_, _, _, result_data_type, _) => result_data_type.is_scalar(),
            _ => false, // Other expressions are conservatively considered non-scalar
        }
    }
}

/// AST node for function/method parameters
#[derive(PartialEq, Eq, Clone, Debug)]
pub struct Parameter<'a> {
    pub name: &'static str,
    pub data_type: DataType<'a>,
}

impl<'a> Parameter<'a> {
    /// Helper to extract the MultiVectorClass from a parameter
    pub fn multi_vector_class(&self) -> &'a MultiVectorClass {
        if let DataType::MultiVector(class) = self.data_type {
            class
        } else {
            unreachable!() // panic if called on a non-MultiVector parameter
        }
    }
}

/// AstNode represents nodes in the abstract syntax tree
#[derive(PartialEq, Eq, Clone)]
pub enum AstNode<'a> {
    None,
    Preamble,
    ClassDefinition {
        class: &'a MultiVectorClass,
    },
    ReturnStatement {
        expression: Box<Expression<'a>>,
    },
    VariableAssignment {
        name: &'static str,
        data_type: Option<DataType<'a>>,
        expression: Box<Expression<'a>>,
    },
    IfThenBlock {
        condition: Box<Expression<'a>>,
        body: Vec<AstNode<'a>>,
    },
    WhileLoopBlock {
        condition: Box<Expression<'a>>,
        body: Vec<AstNode<'a>>,
    },
    TraitImplementation {
        result: Parameter<'a>,
        parameters: Vec<Parameter<'a>>,
        body: Vec<AstNode<'a>>,
    },
}
