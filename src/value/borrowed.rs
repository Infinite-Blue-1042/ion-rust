// Copyright Amazon.com, Inc. or its affiliates.

//! Provides borrowed implementations of [`SymbolToken`], [`Element`] and its dependents.
//!
//! Specifically, all implementations are tied to some particular lifetime, generally linked
//! to a parser implementation of some sort or some context from which the borrow can occur.
//! For simple values, the values are inlined (see [`BorrowedValue`]), but for things that are
//! backed by octets or string data, `&[u8]` and `&str` are used.

use super::{Element, ImportSource, Sequence, Struct, SymbolToken};
use crate::types::decimal::Decimal;
use crate::types::timestamp::Timestamp;
use crate::types::SymbolId;
use crate::value::AnyInt;
use crate::IonType;
use std::collections::HashMap;
use std::iter::FromIterator;

/// A borrowed implementation of [`ImportSource`].
#[derive(Debug, Copy, Clone)]
pub struct BorrowedImportSource<'val> {
    table: &'val str,
    sid: SymbolId,
}

impl<'val> BorrowedImportSource<'val> {
    pub fn new(table: &'val str, sid: SymbolId) -> Self {
        Self { table, sid }
    }
}

impl<'val> ImportSource for BorrowedImportSource<'val> {
    fn table(&self) -> &str {
        self.table
    }

    fn sid(&self) -> usize {
        self.sid
    }
}

/// A borrowed implementation of [`SymbolToken`].
#[derive(Debug, Copy, Clone)]
pub struct BorrowedSymbolToken<'val> {
    text: Option<&'val str>,
    local_sid: Option<SymbolId>,
    source: Option<BorrowedImportSource<'val>>,
}

impl<'val> BorrowedSymbolToken<'val> {
    pub fn new(
        text: Option<&'val str>,
        local_sid: Option<SymbolId>,
        source: Option<BorrowedImportSource<'val>>,
    ) -> Self {
        Self {
            text,
            local_sid,
            source,
        }
    }
}

impl<'val> From<&'val str> for BorrowedSymbolToken<'val> {
    fn from(text: &'val str) -> Self {
        Self::new(Some(text), None, None)
    }
}

impl<'val> SymbolToken for BorrowedSymbolToken<'val> {
    type ImportSource = BorrowedImportSource<'val>;

    fn text(&self) -> Option<&str> {
        self.text
    }

    fn local_sid(&self) -> Option<usize> {
        self.local_sid
    }

    fn source(&self) -> Option<&Self::ImportSource> {
        self.source.as_ref()
    }
}

/// A borrowed implementation of [`Sequence`].
#[derive(Debug, Clone)]
pub struct BorrowedSequence<'val> {
    children: Vec<BorrowedElement<'val>>,
}

impl<'val> BorrowedSequence<'val> {
    pub fn new(children: Vec<BorrowedElement<'val>>) -> Self {
        Self { children }
    }
}

impl<'val> Sequence for BorrowedSequence<'val> {
    type Element = BorrowedElement<'val>;

    fn iter<'a>(&'a self) -> Box<dyn Iterator<Item = &'a Self::Element> + 'a> {
        Box::new(self.children.iter())
    }

    fn get(&self, index: usize) -> Option<&Self::Element> {
        if index > self.children.len() {
            None
        } else {
            Some(&self.children[index])
        }
    }

    fn len(&self) -> usize {
        self.children.len()
    }

    fn is_empty(&self) -> bool {
        self.children.len() == 0
    }
}

/// A borrowed implementation of [`Struct`]
#[derive(Debug, Clone)]
pub struct BorrowedStruct<'val> {
    text_fields: HashMap<String, Vec<(BorrowedSymbolToken<'val>, BorrowedElement<'val>)>>,
    no_text_fields: Vec<(BorrowedSymbolToken<'val>, BorrowedElement<'val>)>,
}

impl<'val, K, V> FromIterator<(K, V)> for BorrowedStruct<'val>
where
    K: Into<BorrowedSymbolToken<'val>>,
    V: Into<BorrowedElement<'val>>,
{
    /// Returns a borrowed struct from the given iterator of field names/values.
    fn from_iter<I: IntoIterator<Item = (K, V)>>(iter: I) -> Self {
        let mut text_fields: HashMap<String, Vec<(BorrowedSymbolToken, BorrowedElement)>> =
            HashMap::new();
        let mut no_text_fields: Vec<(BorrowedSymbolToken, BorrowedElement)> = Vec::new();

        for (k, v) in iter {
            let key = k.into();
            let val = v.into();

            match key.text() {
                Some(text) => {
                    let vals = text_fields.entry(text.into()).or_insert(Vec::new());
                    vals.push((key, val));
                }
                None => {
                    no_text_fields.push((key, val));
                }
            }
        }

        Self {
            text_fields,
            no_text_fields,
        }
    }
}

impl<'val> Struct for BorrowedStruct<'val> {
    type FieldName = BorrowedSymbolToken<'val>;
    type Element = BorrowedElement<'val>;

    fn iter<'a>(
        &'a self,
    ) -> Box<dyn Iterator<Item = (&'a Self::FieldName, &'a Self::Element)> + 'a> {
        // flattens the text_fields HashMap and chains with no_text_fields
        // to return all fields with iterator
        Box::new(
            self.text_fields
                .values()
                .flat_map(|v| v)
                .into_iter()
                .chain(self.no_text_fields.iter())
                .map(|(s, v)| (s, v)),
        )
    }

    fn get<T: AsRef<str>>(&self, field_name: T) -> Option<&Self::Element> {
        self.text_fields
            .get(field_name.as_ref())?
            .last()
            .map(|(_s, v)| v)
    }

    fn get_all<'a, T: AsRef<str>>(
        &'a self,
        field_name: T,
    ) -> Box<dyn Iterator<Item = &'a Self::Element> + 'a> {
        Box::new(
            self.text_fields
                .get(field_name.as_ref())
                .into_iter()
                .flat_map(|v| v.iter())
                .map(|(_s, v)| v),
        )
    }
}

// TODO replace the references with `Cow` and bridge to the owned APIs for mutability

/// Variants for all borrowed version _values_ within an [`Element`].
#[derive(Debug, Clone)]
pub enum BorrowedValue<'val> {
    Null(IonType),
    Integer(AnyInt),
    Float(f64),
    Decimal(Decimal),
    Timestamp(Timestamp),
    String(&'val str),
    Symbol(BorrowedSymbolToken<'val>),
    Boolean(bool),
    Blob(&'val [u8]),
    Clob(&'val [u8]),
    SExpression(BorrowedSequence<'val>),
    List(BorrowedSequence<'val>),
    Struct(BorrowedStruct<'val>),
    // TODO fill this in with the rest...
}

/// A borrowed implementation of [`Element`]
#[derive(Debug, Clone)]
pub struct BorrowedElement<'val> {
    annotations: Vec<BorrowedSymbolToken<'val>>,
    value: BorrowedValue<'val>,
}

impl<'val> BorrowedElement<'val> {
    pub fn new(annotations: Vec<BorrowedSymbolToken<'val>>, value: BorrowedValue<'val>) -> Self {
        Self { annotations, value }
    }
}

impl<'val> From<BorrowedValue<'val>> for BorrowedElement<'val> {
    /// Constructs a [`BorrowedElement`] without annotations from this value.
    fn from(val: BorrowedValue<'val>) -> Self {
        Self::new(vec![], val)
    }
}

impl<'val> Element for BorrowedElement<'val> {
    type SymbolToken = BorrowedSymbolToken<'val>;
    type Sequence = BorrowedSequence<'val>;
    type Struct = BorrowedStruct<'val>;

    fn ion_type(&self) -> IonType {
        use BorrowedValue::*;
        match &self.value {
            Null(t) => *t,
            Integer(_) => IonType::Integer,
            Float(_) => IonType::Float,
            Decimal(_) => IonType::Decimal,
            Timestamp(_) => IonType::Timestamp,
            String(_) => IonType::String,
            Symbol(_) => IonType::Symbol,
            Boolean(_) => IonType::Boolean,
            Blob(_) => IonType::Blob,
            Clob(_) => IonType::Clob,
            SExpression(_) => IonType::SExpression,
            List(_) => IonType::List,
            Struct(_) => IonType::Struct,
        }
    }

    fn annotations<'a>(&'a self) -> Box<dyn Iterator<Item = &'a Self::SymbolToken> + 'a> {
        Box::new(self.annotations.iter())
    }

    fn is_null(&self) -> bool {
        match &self.value {
            BorrowedValue::Null(_) => true,
            _ => false,
        }
    }

    fn as_any_int(&self) -> Option<&AnyInt> {
        match &self.value {
            BorrowedValue::Integer(i) => Some(i),
            _ => None,
        }
    }

    fn as_f64(&self) -> Option<f64> {
        match &self.value {
            BorrowedValue::Float(f) => Some(*f),
            _ => None,
        }
    }

    fn as_decimal(&self) -> Option<&Decimal> {
        match &self.value {
            BorrowedValue::Decimal(d) => Some(d),
            _ => None,
        }
    }

    fn as_timestamp(&self) -> Option<&Timestamp> {
        match &self.value {
            BorrowedValue::Timestamp(t) => Some(t),
            _ => None,
        }
    }

    fn as_str(&self) -> Option<&str> {
        match &self.value {
            BorrowedValue::String(text) => Some(*text),
            BorrowedValue::Symbol(sym) => sym.text(),
            _ => None,
        }
    }

    fn as_sym(&self) -> Option<&Self::SymbolToken> {
        match &self.value {
            BorrowedValue::Symbol(sym) => Some(sym),
            _ => None,
        }
    }

    fn as_bool(&self) -> Option<bool> {
        match &self.value {
            BorrowedValue::Boolean(b) => Some(*b),
            _ => None,
        }
    }

    fn as_bytes(&self) -> Option<&[u8]> {
        match &self.value {
            BorrowedValue::Blob(bytes) | BorrowedValue::Clob(bytes) => Some(*bytes),
            _ => None,
        }
    }

    fn as_sequence(&self) -> Option<&Self::Sequence> {
        match &self.value {
            BorrowedValue::SExpression(seq) | BorrowedValue::List(seq) => Some(seq),
            _ => None,
        }
    }

    fn as_struct(&self) -> Option<&Self::Struct> {
        match &self.value {
            BorrowedValue::Struct(structure) => Some(structure),
            _ => None,
        }
    }
}
