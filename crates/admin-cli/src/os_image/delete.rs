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
    use clap::Parser;

    #[derive(Parser, Debug, Clone)]
    pub struct Args {
        #[clap(short = 'i', long, help = "uuid of the OS image to delete.")]
        pub id: String,
        #[clap(
            short = 't',
            long,
            help = "Tenant organization identifier of OS image to delete."
        )]
        pub tenant_org_id: String,
    }
}

pub mod cmd {
    use ::rpc::forge::DeleteOsImageRequest;

    use super::args::Args;
    use super::*;
    use crate::os_image::common::str_to_rpc_uuid;
    use crate::rpc::ApiClient;

    pub async fn delete(args: Args, api_client: &ApiClient) -> CarbideCliResult<()> {
        let id = str_to_rpc_uuid(&args.id)?;
        api_client
            .0
            .delete_os_image(DeleteOsImageRequest {
                id: Some(id.clone()),
                tenant_organization_id: args.tenant_org_id,
            })
            .await?;
        println!("OS image {id} deleted successfully.");
        Ok(())
    }
}

impl Run for Args {
    async fn run(self, ctx: &mut RuntimeContext) -> CarbideCliResult<()> {
        cmd::delete(self, &ctx.api_client).await
    }
}
