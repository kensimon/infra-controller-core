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

use ::rpc::admin_cli::CarbideCliResult;
pub use args::Args;

use crate::cfg::run::Run;
use crate::cfg::runtime::RuntimeContext;

pub mod args {
    use carbide_uuid::dpu_remediations::RemediationId;
    use clap::Parser;

    #[derive(Parser, Debug)]
    pub struct Args {
        #[clap(help = "The id of the remediation to revoke", long)]
        pub id: RemediationId,
    }
}

pub mod cmd {
    use ::rpc::admin_cli::CarbideCliError;
    use rpc::forge::RevokeRemediationRequest;

    use super::args::Args;
    use crate::rpc::ApiClient;

    pub async fn revoke_dpu_remediation(
        revoke_remediation: Args,
        api_client: &ApiClient,
    ) -> Result<(), CarbideCliError> {
        api_client
            .0
            .revoke_remediation(RevokeRemediationRequest {
                remediation_id: Some(revoke_remediation.id),
            })
            .await?;

        tracing::info!("Revoked remediation with id: {:?}", revoke_remediation.id);
        Ok(())
    }
}

impl Run for Args {
    async fn run(self, ctx: &mut RuntimeContext) -> CarbideCliResult<()> {
        cmd::revoke_dpu_remediation(self, &ctx.api_client).await
    }
}
