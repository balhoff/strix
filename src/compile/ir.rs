use crate::dict::TermId;

// ── Compiled SWRL types ──────────────────────────────────────────────

/// A compiled SWRL rule ready for evaluation.
#[derive(Clone, Debug)]
pub struct CompiledSwrlRule {
    /// Index of the trigger atom in `body`.
    pub trigger: usize,
    /// Body atom indices excluding the trigger, pre-computed for evaluation.
    pub remaining: Vec<usize>,
    pub body: Vec<SwrlBodyAtom>,
    pub head: SwrlHeadAtom,
    /// Total number of distinct variables across this rule.
    pub num_vars: u32,
}

#[derive(Clone, Debug)]
pub enum SwrlBodyAtom {
    ClassAtom {
        class: TermId,
        arg: SwrlArg,
    },
    PropertyAtom {
        property: TermId,
        subject: SwrlArg,
        object: SwrlArg,
    },
    SameIndividualAtom {
        left: SwrlArg,
        right: SwrlArg,
    },
    DifferentIndividualsAtom {
        left: SwrlArg,
        right: SwrlArg,
    },
}

#[derive(Clone, Debug)]
pub enum SwrlHeadAtom {
    ClassAtom {
        class: TermId,
        arg: SwrlArg,
    },
    PropertyAtom {
        property: TermId,
        subject: SwrlArg,
        object: SwrlArg,
    },
    SameIndividualAtom {
        left: SwrlArg,
        right: SwrlArg,
    },
    DifferentIndividualsAtom {
        left: SwrlArg,
        right: SwrlArg,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SwrlArg {
    Variable(u32),
    Constant(TermId),
}

impl SwrlArg {
    pub fn as_variable(self) -> Option<u32> {
        match self {
            Self::Variable(v) => Some(v),
            Self::Constant(_) => None,
        }
    }
}
