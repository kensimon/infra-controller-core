use sqlx::{Postgres, QueryBuilder};

#[derive(Default)]
pub struct ColumnSet<'a> {
    set: Vec<(&'static str, ColumnValue<'a>)>,
}

impl<'a> ColumnSet<'a> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push<V: Into<ColumnValue<'a>>>(&mut self, column: &'static str, value: V) {
        self.set.push((column, value.into()));
    }

    pub fn push_if_some<V: Into<ColumnValue<'a>>>(
        &mut self,
        column: &'static str,
        value: Option<V>,
    ) {
        if let Some(value) = value {
            self.set.push((column, value.into()));
        }
    }

    fn into_parts(self) -> (Vec<&'static str>, Vec<ColumnValue<'a>>) {
        self.set.into_iter().unzip()
    }
}

pub trait PushValuesForInsert<'a> {
    fn push_values_for_insert(&mut self, column_set: ColumnSet<'a>);
}

impl<'a> PushValuesForInsert<'a> for QueryBuilder<'a, Postgres> {
    fn push_values_for_insert(&mut self, column_set: ColumnSet<'a>) {
        let (columns, values) = column_set.into_parts();

        self.push(" (");
        let mut columns_qb = self.separated(", ");
        for column in columns {
            columns_qb.push(column);
        }

        self.push(") VALUES (");

        let mut values_qb = self.separated(", ");
        for value in values {
            match value {
                ColumnValue::Bool(b) => {
                    values_qb.push_bind(b);
                }
                ColumnValue::String(s) => {
                    values_qb.push_bind(s);
                }
                ColumnValue::Strings(s) => {
                    values_qb.push_bind(s);
                }
                ColumnValue::I64(i) => {
                    values_qb.push_bind(i);
                }
            }
        }
        self.push(")");
    }
}

pub enum ColumnValue<'a> {
    Bool(bool),
    String(&'a str),
    Strings(&'a Vec<String>),
    I64(i64),
}

impl<'a> From<&'a str> for ColumnValue<'a> {
    fn from(value: &'a str) -> Self {
        Self::String(value)
    }
}

impl<'a> From<&'a Vec<String>> for ColumnValue<'a> {
    fn from(value: &'a Vec<String>) -> Self {
        Self::Strings(value)
    }
}

impl<'a> From<bool> for ColumnValue<'a> {
    fn from(value: bool) -> Self {
        Self::Bool(value)
    }
}

impl<'a> From<i64> for ColumnValue<'a> {
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
