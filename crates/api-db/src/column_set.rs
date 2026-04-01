use std::borrow::Cow;

use sqlx::{Postgres, QueryBuilder};

#[derive(Default)]
pub struct ColumnSet<'a> {
    set: Vec<(&'static str, ColumnValue<'a>)>,
}

impl<'a> ColumnSet<'a> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push<V: Into<ColumnKind<'a>>>(&mut self, column: &'static str, value: V) {
        self.set.push((
            column,
            ColumnValue {
                wrap: None,
                kind: value.into(),
            },
        ));
    }

    pub fn push_wrapped<V: Into<ColumnKind<'a>>>(
        &mut self,
        column: &'static str,
        value: V,
        column_wrap: ColumnWrap,
    ) {
        self.set.push((
            column,
            ColumnValue {
                wrap: Some(column_wrap),
                kind: value.into(),
            },
        ));
    }

    pub fn push_if_some<V: Into<ColumnKind<'a>>>(
        &mut self,
        column: &'static str,
        value: Option<V>,
    ) {
        if let Some(value) = value {
            self.set.push((
                column,
                ColumnValue {
                    wrap: None,
                    kind: value.into(),
                },
            ));
        }
    }

    pub fn push_wrapped_if_some<V: Into<ColumnKind<'a>>>(
        &mut self,
        column: &'static str,
        value: Option<V>,
        column_wrap: ColumnWrap,
    ) {
        if let Some(value) = value {
            self.set.push((
                column,
                ColumnValue {
                    wrap: Some(column_wrap),
                    kind: value.into(),
                },
            ));
        }
    }

    pub fn is_empty(&self) -> bool {
        self.set.is_empty()
    }

    fn into_parts(self) -> (Vec<&'static str>, Vec<ColumnValue<'a>>) {
        self.set.into_iter().unzip()
    }
}

pub trait PushValuesForInsert<'a> {
    fn push_values_for_insert(&mut self, column_set: ColumnSet<'a>) -> &mut Self;
}

impl<'a> PushValuesForInsert<'a> for QueryBuilder<'a, Postgres> {
    fn push_values_for_insert(&mut self, column_set: ColumnSet<'a>) -> &mut Self {
        let (columns, values) = column_set.into_parts();

        self.push(" (");
        let mut columns_qb = self.separated(", ");
        for column in columns {
            columns_qb.push(column);
        }

        self.push(") VALUES (");

        let mut values_qb = self.separated(", ");
        for value in values {
            match value.kind {
                ColumnKind::Bool(b) => {
                    values_qb.push_bind(b);
                }
                ColumnKind::String(s) => {
                    values_qb.push_bind(s);
                }
                ColumnKind::Strings(s) => {
                    values_qb.push_bind(s);
                }
                ColumnKind::I64(i) => {
                    values_qb.push_bind(i);
                }
            }
        }
        self.push(")");
        self
    }
}

pub trait PushValuesForUpdate<'a> {
    fn push_values_for_update(&mut self, column_set: ColumnSet<'a>) -> &mut Self;
}

impl<'a> PushValuesForUpdate<'a> for QueryBuilder<'a, Postgres> {
    fn push_values_for_update(&mut self, column_set: ColumnSet<'a>) -> &mut Self {
        self.push(" SET ");
        let mut sets = self.separated(", ");

        for (column, value) in column_set.set {
            sets.push_unseparated(column);
            sets.push_unseparated(" = ");
            match value.kind {
                ColumnKind::Bool(v) => {
                    sets.push_bind(v);
                }
                ColumnKind::String(v) => {
                    sets.push_bind(v);
                }
                ColumnKind::Strings(v) => {
                    sets.push_bind(v);
                }
                ColumnKind::I64(v) => {
                    sets.push_bind(v);
                }
            }
        }

        self
    }
}

pub trait PushAsWhereClause<'a> {
    fn push_as_where_clause(&mut self, column_set: ColumnSet<'a>) -> &mut Self;
}

impl<'a> PushAsWhereClause<'a> for QueryBuilder<'a, Postgres> {
    fn push_as_where_clause(&mut self, column_set: ColumnSet<'a>) -> &mut Self {
        for (idx, (column, value)) in column_set.set.into_iter().enumerate() {
            match idx {
                0 => {
                    self.push(" WHERE ");
                }
                _ => {
                    self.push(" AND ");
                }
            }

            let op = match &value.kind {
                ColumnKind::Strings(_) => "&&",
                _ => "=",
            };

            if matches!(value.wrap, Some(ColumnWrap::ToLower)) {
                self.push(format_args!("LOWER({column}) {op} LOWER("));
            } else {
                self.push(format_args!("{column} {op} "));
            }

            match value.kind {
                ColumnKind::Bool(v) => {
                    self.push_bind(v);
                }
                ColumnKind::String(v) => {
                    self.push_bind(v);
                }
                ColumnKind::Strings(v) => {
                    self.push_bind(v);
                }
                ColumnKind::I64(v) => {
                    self.push_bind(v);
                }
            }

            if matches!(value.wrap, Some(ColumnWrap::ToLower)) {
                self.push(")");
            }
        }

        self
    }
}

pub struct ColumnValue<'a> {
    wrap: Option<ColumnWrap>,
    kind: ColumnKind<'a>,
}

pub enum ColumnWrap {
    ToLower,
}

pub enum ColumnKind<'a> {
    Bool(bool),
    String(Cow<'a, str>),
    Strings(&'a Vec<String>),
    I64(i64),
}

impl<'a> From<&'a str> for ColumnKind<'a> {
    fn from(value: &'a str) -> Self {
        Self::String(Cow::Borrowed(value))
    }
}

impl<'a> From<String> for ColumnKind<'a> {
    fn from(value: String) -> Self {
        Self::String(Cow::Owned(value))
    }
}

impl<'a> From<&'a Vec<String>> for ColumnKind<'a> {
    fn from(value: &'a Vec<String>) -> Self {
        Self::Strings(value)
    }
}

impl<'a> From<bool> for ColumnKind<'a> {
    fn from(value: bool) -> Self {
        Self::Bool(value)
    }
}

impl<'a> From<i64> for ColumnKind<'a> {
    fn from(value: i64) -> Self {
        Self::I64(value)
    }
}

pub trait IfNonEmpty {
    fn if_non_empty(&self) -> Option<&Self>;
}

impl IfNonEmpty for Vec<String> {
    fn if_non_empty(&self) -> Option<&Self> {
        if self.is_empty() { None } else { Some(&self) }
    }
}
