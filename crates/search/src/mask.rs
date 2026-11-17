//! Eligibility masks: which documents a book selection allows, by document ordinal.
//!
//! A selection decides eligibility once. Every search, count and correction that follows reads
//! the answer by ordinal, so a keystroke does not look up 31 thousand ids again.

use crate::{DocView, SearchIndex};

/// The documents that one selection allows, indexed by document ordinal. Built by
/// [`SearchIndex::eligible_mask`] for one index. Reusing it with a different index is a
/// mistake: a mask that is too short reads as "not allowed" for the missing ordinals.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EligibleMask {
    allowed: Vec<bool>,
    count: usize,
}

impl EligibleMask {
    fn from_flags(allowed: Vec<bool>) -> Self {
        let count = allowed.iter().filter(|&&ok| ok).count();
        Self { allowed, count }
    }

    /// Whether the document at `ord` is allowed.
    #[inline]
    pub fn allows(&self, ord: usize) -> bool {
        self.allowed.get(ord).copied().unwrap_or(false)
    }

    /// The number of allowed documents.
    pub fn allowed_count(&self) -> usize {
        self.count
    }

    /// The number of documents the mask covers, allowed or not.
    pub fn len(&self) -> usize {
        self.allowed.len()
    }

    pub fn is_empty(&self) -> bool {
        self.allowed.is_empty()
    }

    /// True when every document of the index is allowed.
    pub fn allows_all(&self) -> bool {
        self.count == self.allowed.len()
    }
}

impl SearchIndex {
    /// Evaluates `accept` once for every document and records the answers. The caller keeps the
    /// mask for as long as its book selection stays the same.
    pub fn eligible_mask(&self, accept: impl Fn(DocView<'_>) -> bool) -> EligibleMask {
        EligibleMask::from_flags(
            (0..self.meta.docs.len())
                .map(|ord| accept(self.view(ord)))
                .collect(),
        )
    }

    /// The mask for ids only. Equal to [`SearchIndex::eligible_mask`] over `eligible(doc.id)`.
    pub fn eligible_mask_ids(&self, eligible: impl Fn(&str) -> bool) -> EligibleMask {
        self.eligible_mask(|doc| eligible(doc.id))
    }

    /// `mask` with the documents that `keep` rejects removed. `keep` runs only for documents the
    /// mask already allows.
    pub fn narrow_mask(
        &self,
        mask: &EligibleMask,
        keep: impl Fn(DocView<'_>) -> bool,
    ) -> EligibleMask {
        EligibleMask::from_flags(
            (0..self.meta.docs.len())
                .map(|ord| mask.allows(ord) && keep(self.view(ord)))
                .collect(),
        )
    }
}
