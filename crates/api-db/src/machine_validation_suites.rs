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

use config_version::ConfigVersion;
use model::machine_validation::{
    MachineValidationTest, MachineValidationTestAddRequest, MachineValidationTestUpdatePayload,
    MachineValidationTestUpdateRequest, MachineValidationTestsGetRequest,
};
use regex::Regex;
use sqlx::{Execute, PgConnection, Postgres, QueryBuilder};

use crate::column_set::{ColumnSet, IfNonEmpty, PushValuesForInsert, PushValuesForUpdate};
use crate::db_read::DbReader;
use crate::{DatabaseError, DatabaseResult};

const MVT_TABLE: &str = "machine_validation_tests";

fn push_select_filters<'a>(
    qb: &mut QueryBuilder<'a, Postgres>,
    req: &'a MachineValidationTestsGetRequest,
) {
    if let Some(ref tid) = req.test_id {
        qb.push(" AND LOWER(test_id) = LOWER(");
        qb.push_bind(tid);
        qb.push(")");
    }
    if let Some(ref v) = req.version {
        qb.push(" AND version = ");
        qb.push_bind(v);
    }
    if let Some(b) = req.is_enabled {
        qb.push(" AND is_enabled = ");
        qb.push_bind(b);
    }
    if let Some(b) = req.verified {
        qb.push(" AND verified = ");
        qb.push_bind(b);
    }
    if let Some(b) = req.read_only {
        qb.push(" AND read_only = ");
        qb.push_bind(b);
    }
    if !req.supported_platforms.is_empty() {
        qb.push(" AND supported_platforms && ");
        qb.push_bind(&req.supported_platforms);
    }
    if !req.contexts.is_empty() {
        qb.push(" AND contexts && ");
        qb.push_bind(&req.contexts);
    }
    if !req.custom_tags.is_empty() {
        qb.push(" AND custom_tags && ");
        qb.push_bind(&req.custom_tags);
    }
}

pub async fn find(
    txn: impl DbReader<'_>,
    req: MachineValidationTestsGetRequest,
) -> DatabaseResult<Vec<MachineValidationTest>> {
    let mut qb = QueryBuilder::new("SELECT * FROM ");
    qb.push(MVT_TABLE);
    qb.push(" WHERE 1=1");
    push_select_filters(&mut qb, &req);
    qb.push(" ORDER BY version DESC, name ASC");
    let q = qb.build_query_as::<MachineValidationTest>();
    let sql = q.sql();
    q.fetch_all(txn)
        .await
        .map_err(|e| DatabaseError::query(sql, e))
}

pub fn generate_test_id(name: &str) -> String {
    format!("forge_{}", name.to_ascii_lowercase())
}

/// INSERT semantics match the previous serde_json-driven builder: skip `Option::None`, skip empty
/// `Vec`s for array columns, always set `version`, `test_id`, `modified_by`.

pub async fn save(
    txn: &mut PgConnection,
    mut req: MachineValidationTestAddRequest,
    version: ConfigVersion,
) -> DatabaseResult<String> {
    let test_id = generate_test_id(&req.name);

    let re = Regex::new(r"[ =;:@#\!?\-]").unwrap();
    req.supported_platforms = req
        .supported_platforms
        .iter()
        .map(|p| re.replace_all(p, "_").to_string().to_ascii_lowercase())
        .collect();
    let version_string = version.version_string();

    let mut cols = ColumnSet::default();
    cols.push("name", req.name.as_str());
    cols.push("command", req.command.as_str());
    cols.push("args", req.args.as_str());
    cols.push("version", version_string.as_str());
    cols.push("test_id", test_id.as_str());
    cols.push("modified_by", "User");

    cols.push_if_some("description", req.description.as_deref());
    cols.push_if_some("contexts", req.contexts.if_non_empty());
    cols.push_if_some("img_name", req.img_name.as_deref());
    cols.push_if_some("execute_in_host", req.execute_in_host);
    cols.push_if_some("container_arg", req.container_arg.as_deref());
    cols.push_if_some("extra_err_file", req.extra_err_file.as_deref());
    cols.push_if_some("external_config_file", req.external_config_file.as_deref());
    cols.push_if_some("pre_condition", req.pre_condition.as_deref());
    cols.push_if_some("timeout", req.timeout);
    cols.push_if_some("extra_output_file", req.extra_output_file.as_deref());
    cols.push_if_some(
        "supported_platforms",
        req.supported_platforms.if_non_empty(),
    );
    cols.push_if_some("read_only", req.read_only);
    cols.push_if_some("custom_tags", req.custom_tags.if_non_empty());
    cols.push_if_some("components", req.components.if_non_empty());
    cols.push_if_some("is_enabled", req.is_enabled);

    let mut qb = QueryBuilder::new("INSERT INTO ");
    qb.push(MVT_TABLE);
    qb.push_values_for_insert(cols);
    qb.push(" RETURNING test_id");

    let q = qb.build_query_scalar::<String>();
    let sql = q.sql();
    let returned = q
        .fetch_one(&mut *txn)
        .await
        .map_err(|e| DatabaseError::query(sql, e))?;
    debug_assert_eq!(returned, test_id);
    Ok(test_id)
}

/// UPDATE: at least one non-verified field or explicit `verified` must be present, or
/// `InvalidArgument("Nothing to update")`. If `verified` is omitted, it is set to `false` after
/// other columns are applied (same as the legacy JSON builder).
pub async fn update(
    txn: &mut PgConnection,
    req: MachineValidationTestUpdateRequest,
) -> DatabaseResult<String> {
    let Some(mut payload) = req.payload else {
        return Err(DatabaseError::InvalidArgument(
            "Payload is missing".to_owned(),
        ));
    };
    let re = Regex::new(r"[ =;:@#\!?\-]").unwrap();
    payload.supported_platforms = payload
        .supported_platforms
        .iter()
        .map(|p| re.replace_all(p, "_").to_string().to_ascii_lowercase())
        .collect();

    let mut columns = ColumnSet::new();
    columns.push_if_some("name", payload.name.as_deref());
    columns.push_if_some("description", payload.description.as_deref());
    columns.push_if_some("contexts", payload.contexts.if_non_empty());
    columns.push_if_some("img_name", payload.img_name.as_deref());
    columns.push_if_some("execute_in_host", payload.execute_in_host);
    columns.push_if_some("container_arg", payload.container_arg.as_deref());
    columns.push_if_some("command", payload.command.as_deref());
    columns.push_if_some("args", payload.args.as_deref());
    columns.push_if_some("extra_err_file", payload.extra_err_file.as_deref());
    columns.push_if_some(
        "external_config_file",
        payload.external_config_file.as_deref(),
    );
    columns.push_if_some("pre_condition", payload.pre_condition.as_deref());
    columns.push_if_some("timeout", payload.timeout);
    columns.push_if_some("extra_output_file", payload.extra_output_file.as_deref());
    columns.push_if_some(
        "supported_platforms",
        payload.supported_platforms.if_non_empty(),
    );
    columns.push_if_some("verified", payload.verified);
    columns.push_if_some("custom_tags", payload.custom_tags.if_non_empty());
    columns.push_if_some("components", payload.components.if_non_empty());
    columns.push_if_some("is_enabled", payload.is_enabled);

    if columns.is_empty() {
        return Err(DatabaseError::InvalidArgument(
            "Nothing to update".to_string(),
        ));
    }

    if payload.verified.is_none() {
        columns.push("verified", false);
    }
    columns.push("modified_by", "User");

    let mut qb = QueryBuilder::new("UPDATE ");
    let q = qb
        .push(MVT_TABLE)
        .push_values_for_update(columns)
        .push(" WHERE test_id = ")
        .push_bind(&req.test_id)
        .push(" AND version = ")
        .push_bind(&req.version)
        .push(" RETURNING test_id")
        .build_query_scalar::<String>();
    let sql = q.sql();
    q.fetch_one(&mut *txn)
        .await
        .map_err(|e| DatabaseError::query(sql, e))?;
    Ok(req.test_id)
}

pub async fn clone(
    txn: &mut PgConnection,
    test: &MachineValidationTest,
) -> DatabaseResult<(String, ConfigVersion)> {
    let add_req = MachineValidationTestAddRequest {
        name: test.name.clone(),
        description: test.description.clone(),
        contexts: test.contexts.clone(),
        img_name: test.img_name.clone(),
        execute_in_host: test.execute_in_host,
        container_arg: test.container_arg.clone(),
        command: test.command.clone(),
        args: test.args.clone(),
        extra_err_file: test.extra_err_file.clone(),
        external_config_file: test.external_config_file.clone(),
        pre_condition: test.pre_condition.clone(),
        timeout: test.timeout,
        extra_output_file: test.extra_output_file.clone(),
        supported_platforms: test.supported_platforms.clone(),
        read_only: None,
        custom_tags: test.custom_tags.clone().unwrap_or_default(),
        components: test.components.clone(),
        is_enabled: Some(test.is_enabled),
    };
    let next_version = test.version.increment();
    let test_id = save(txn, add_req, next_version).await?;
    Ok((test_id, next_version))
}

pub async fn mark_verified(
    txn: &mut PgConnection,
    test_id: String,
    version: ConfigVersion,
) -> DatabaseResult<String> {
    let req = MachineValidationTestUpdateRequest {
        test_id,
        version: version.version_string(),
        payload: Some(MachineValidationTestUpdatePayload {
            verified: Some(true),
            ..Default::default()
        }),
    };
    update(txn, req).await
}

pub async fn enable_disable(
    txn: &mut PgConnection,
    test_id: String,
    version: ConfigVersion,
    is_enabled: bool,
    is_verified: bool,
) -> DatabaseResult<String> {
    let req = MachineValidationTestUpdateRequest {
        test_id,
        version: version.version_string(),
        payload: Some(MachineValidationTestUpdatePayload {
            is_enabled: Some(is_enabled),
            verified: Some(is_verified),
            ..Default::default()
        }),
    };
    update(txn, req).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_test_id_lowercases_name() {
        assert_eq!(generate_test_id("MyTest"), "forge_mytest");
        assert_eq!(generate_test_id("ALLCAPS"), "forge_allcaps");
        assert_eq!(generate_test_id("already_lower"), "forge_already_lower");
        assert_eq!(generate_test_id("MiXeD_CaSe_123"), "forge_mixed_case_123");
    }

    #[test]
    fn select_query_uses_lower_for_test_id_and_placeholders() {
        let req = MachineValidationTestsGetRequest {
            test_id: Some("Forge_MyTest".to_string()),
            ..Default::default()
        };
        let mut qb = QueryBuilder::new("SELECT * FROM ");
        qb.push(MVT_TABLE);
        qb.push(" WHERE 1=1");
        push_select_filters(&mut qb, &req);
        qb.push(" ORDER BY version DESC, name ASC");
        let sql = qb.build().sql();
        assert!(
            sql.contains("LOWER(test_id)"),
            "Expected LOWER(test_id), got: {sql}"
        );
        assert!(
            sql.contains("LOWER(") && sql.contains(')'),
            "Expected bound LOWER comparison, got: {sql}"
        );
    }

    #[test]
    fn select_query_boolean_uses_placeholder() {
        let req = MachineValidationTestsGetRequest {
            is_enabled: Some(true),
            ..Default::default()
        };
        let mut qb = QueryBuilder::new("SELECT * FROM ");
        qb.push(MVT_TABLE);
        qb.push(" WHERE 1=1");
        push_select_filters(&mut qb, &req);
        let sql = qb.build().sql();
        assert!(
            sql.contains("is_enabled = $"),
            "Expected parameterized is_enabled, got: {sql}"
        );
    }

    #[test]
    fn select_query_empty_request_is_select_all() {
        let req = MachineValidationTestsGetRequest::default();
        let mut qb = QueryBuilder::new("SELECT * FROM ");
        qb.push(MVT_TABLE);
        qb.push(" WHERE 1=1");
        push_select_filters(&mut qb, &req);
        let sql = qb.build().sql();
        assert!(
            sql.contains("WHERE 1=1"),
            "Empty request should have no extra filters, got: {sql}"
        );
        assert!(
            !sql.contains("LOWER(test_id)"),
            "Empty request should not filter test_id, got: {sql}"
        );
    }
}
