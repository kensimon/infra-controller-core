/*
 * SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
 * SPDX-License-Identifier: Apache-2.0
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 * http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

use std::fmt;
use std::fmt::{Debug, Display, Formatter};
use std::str::FromStr;

use prost::DecodeError;
use prost::bytes::{Buf, BufMut};
use prost::encoding::{DecodeContext, WireType};
use serde::{Deserialize, Serialize};
#[cfg(feature = "sqlx")]
use sqlx::{
    encode::IsNull,
    error::BoxDynError,
    postgres::{PgHasArrayType, PgTypeInfo},
    {Database, Postgres, Row},
};

use crate::machine::{MachineId, MachineIdParseError};
use crate::power_shelf::{PowerShelfId, PowerShelfIdParseError};
use crate::switch::{SwitchId, SwitchIdParseError};

#[derive(Copy, Clone, Eq, Hash, PartialEq, Ord, PartialOrd)]
pub enum DeviceId {
    Machine(MachineId),
    Switch(SwitchId),
    PowerShelf(PowerShelfId),
}

// Implement [`prost::Message`] manually so that we can be wire-compatible with the
// `.common.DeviceId` protobuf message, which is what we actually serialize. Do this by
// constructing a `legacy_rpc::DeviceId` and delegate all  [`prost::Message`] methods to it.
impl prost::Message for DeviceId {
    fn encode_raw(&self, buf: &mut impl BufMut)
    where
        Self: Sized,
    {
        legacy_rpc::DeviceId::from(*self).encode_raw(buf);
    }

    fn merge_field(
        &mut self,
        tag: u32,
        wire_type: WireType,
        buf: &mut impl Buf,
        ctx: DecodeContext,
    ) -> Result<(), DecodeError>
    where
        Self: Sized,
    {
        let mut legacy_message = legacy_rpc::DeviceId::from(*self);
        legacy_message.merge_field(tag, wire_type, buf, ctx)?;
        *self = DeviceId::from_str(&legacy_message.id).map_err(|_| {
            // Deprecation: if they remove DecodeError::new, they hopefully will provide some other way
            // to impl prost::Message.
            #[allow(deprecated)]
            DecodeError::new(format!("Invalid machine id: {}", legacy_message.id))
        })?;
        Ok(())
    }

    fn encoded_len(&self) -> usize {
        legacy_rpc::DeviceId::from(*self).encoded_len()
    }

    #[allow(deprecated)]
    fn clear(&mut self) {
        *self = DeviceId::default();
    }
}

mod legacy_rpc {
    /// Backwards compatiblity shim for [`super::DeviceId`] to be sent as a protobuf message in a
    /// way that is compatible with the `.common.DeviceId` message, which is defined as:
    ///
    /// ```ignore
    /// message DeviceId {
    ///     oneof value {
    ///         MachineId machine_id = 1;
    ///         SwitchId switch_id = 2;
    ///         PowerShelfId power_shelf_id = 3;
    ///     }
    /// }
    /// ```
    ///
    /// This allows us to use [`super::DeviceId`] directly instead of having to convert it
    /// manually every time, while still interacting with peers that expect a `.common.DeviceId`
    /// to be serialized.
    #[derive(prost::Message)]
    pub struct DeviceId {
        #[prost(string, tag = "1")]
        pub id: String,
    }

    impl From<super::DeviceId> for DeviceId {
        fn from(value: crate::device::DeviceId) -> Self {
            Self {
                id: value.to_string(),
            }
        }
    }
}

impl Default for DeviceId {
    #[allow(deprecated)]
    fn default() -> Self {
        Self::default()
    }
}

impl Debug for DeviceId {
    // The derived Debug implementation is messy, just output the string representation even when
    // debugging.
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(self, f)
    }
}

// Make DeviceId bindable directly into a sqlx query
#[cfg(feature = "sqlx")]
impl sqlx::Encode<'_, sqlx::Postgres> for DeviceId {
    fn encode_by_ref(
        &self,
        buf: &mut <Postgres as Database>::ArgumentBuffer,
    ) -> Result<IsNull, BoxDynError> {
        buf.extend(self.to_string().as_bytes());
        Ok(sqlx::encode::IsNull::No)
    }
}

#[cfg(feature = "sqlx")]
impl<'r, DB> sqlx::Decode<'r, DB> for DeviceId
where
    DB: sqlx::Database,
    String: sqlx::Decode<'r, DB>,
{
    fn decode(
        value: <DB as sqlx::database::Database>::ValueRef<'r>,
    ) -> Result<Self, sqlx::error::BoxDynError> {
        let str_id: String = String::decode(value)?;
        Ok(DeviceId::from_str(&str_id).map_err(|e| sqlx::Error::Decode(Box::new(e)))?)
    }
}

#[cfg(feature = "sqlx")]
impl<'r> sqlx::FromRow<'r, sqlx::postgres::PgRow> for DeviceId {
    fn from_row(row: &'r sqlx::postgres::PgRow) -> Result<Self, sqlx::Error> {
        let id: DeviceId = row.try_get(0)?;
        Ok(id)
    }
}

#[cfg(feature = "sqlx")]
impl<DB> sqlx::Type<DB> for DeviceId
where
    DB: sqlx::Database,
    String: sqlx::Type<DB>,
{
    fn type_info() -> <DB as sqlx::Database>::TypeInfo {
        String::type_info()
    }

    fn compatible(ty: &DB::TypeInfo) -> bool {
        String::compatible(ty)
    }
}

#[cfg(feature = "sqlx")]
impl PgHasArrayType for DeviceId {
    fn array_type_info() -> PgTypeInfo {
        <&str as PgHasArrayType>::array_type_info()
    }

    fn array_compatible(ty: &PgTypeInfo) -> bool {
        <&str as PgHasArrayType>::array_compatible(ty)
    }
}

impl DeviceId {
    /// Note: Never use this! Tonic's codegen requires all types to implement Default, but there is
    /// no logical reason to construct a "default" DeviceId in real code, so we simply construct a
    /// bogus one here.
    #[allow(clippy::should_implement_trait)]
    #[deprecated(
        note = "Do not use `DeviceId::default()` directly; only implemented for prost interop"
    )]
    pub fn default() -> Self {
        #[allow(deprecated)]
        Self::Machine(MachineId::default())
    }
}

impl std::fmt::Display for DeviceId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DeviceId::Machine(m) => <MachineId as Display>::fmt(m, f),
            DeviceId::Switch(s) => <SwitchId as Display>::fmt(s, f),
            DeviceId::PowerShelf(ps) => <PowerShelfId as Display>::fmt(ps, f),
        }
    }
}

#[derive(thiserror::Error, Debug, Clone)]
pub enum DeviceIdParseError {
    #[error("Invalid machine id: {0}")]
    Machine(#[from] MachineIdParseError),
    #[error("Invalid switch id: {0}")]
    Switch(#[from] SwitchIdParseError),
    #[error("Invalid power shelf id: {0}")]
    PowerShelf(#[from] PowerShelfIdParseError),
    #[error("Unable to determine device type from id: {0}")]
    UnknownId(String),
}

impl FromStr for DeviceId {
    type Err = DeviceIdParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if MachineId::is_matching_prefix(s) {
            Ok(Self::Machine(MachineId::from_str(s)?))
        } else if SwitchId::is_matching_prefix(s) {
            Ok(Self::Switch(SwitchId::from_str(s)?))
        } else if PowerShelfId::is_matching_prefix(s) {
            Ok(Self::PowerShelf(PowerShelfId::from_str(s)?))
        } else {
            Err(DeviceIdParseError::UnknownId(s.to_string()))
        }
    }
}

impl Serialize for DeviceId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for DeviceId {
    fn deserialize<D>(deserializer: D) -> Result<DeviceId, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::Error;

        let str_value = String::deserialize(deserializer)?;
        let id = DeviceId::from_str(&str_value).map_err(|err| Error::custom(err.to_string()))?;
        Ok(id)
    }
}
