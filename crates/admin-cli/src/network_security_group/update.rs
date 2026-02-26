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
        #[clap(short = 'i', long, help = "Network security group ID to update")]
        pub id: String,

        #[clap(
            short = 't',
            long,
            help = "Tenant organization ID of the network security group"
        )]
        pub tenant_organization_id: String,

        #[clap(short = 'n', long, help = "Name of the network security group")]
        pub name: Option<String>,

        #[clap(short = 'd', long, help = "Description of the network security group")]
        pub description: Option<String>,

        #[clap(
            short = 'l',
            long,
            help = "JSON map of simple key:value pairs to be applied as labels to the network security group - will COMPLETELY overwrite any existing labels"
        )]
        pub labels: Option<String>,

        #[clap(
            short = 's',
            long,
            help = "Optional, whether egress rules are stateful"
        )]
        pub stateful_egress: Option<bool>,

        #[clap(
            short = 'r',
            long,
            help = "Optional, JSON array containing a defined set of network security group rules - will COMPLETELY overwrite any existing rules"
        )]
        pub rules: Option<String>,

        #[clap(
            short = 'v',
            long,
            help = "Optional, version to use for comparison when performing the update, which will be rejected if the actual version of the record does not match the value of this parameter"
        )]
        pub version: Option<String>,
    }
}

pub mod cmd {
    use ::rpc::admin_cli::{CarbideCliError, OutputFormat};

    use super::args::Args;
    use super::*;
    use crate::network_security_group::common::convert_nsgs_to_table;
    use crate::rpc::ApiClient;

    /// Update a network security group.
    /// On successful update, the details of the
    /// group will be displayed.
    pub async fn update(
        args: Args,
        output_format: OutputFormat,
        api_client: &ApiClient,
    ) -> CarbideCliResult<()> {
        let is_json = output_format == OutputFormat::Json;

        let id = args.id;

        let nsg = api_client
            .get_single_network_security_group(id.clone())
            .await?;

        let mut metadata = nsg.metadata.unwrap_or_default();
        let (mut rules, mut stateful_egress) = {
            let nsg = nsg.attributes.unwrap_or_default();
            (nsg.rules, nsg.stateful_egress)
        };

        if let Some(d) = args.description {
            metadata.description = d;
        }

        if let Some(n) = args.name {
            metadata.name = n;
        }

        if let Some(l) = args.labels {
            metadata.labels = serde_json::from_str(&l)?;
        }

        if let Some(r) = args.rules {
            rules = serde_json::from_str(&r)?;
        }

        if let Some(s) = args.stateful_egress {
            stateful_egress = s;
        }

        let nsg = api_client
            .update_network_security_group(
                id,
                args.tenant_organization_id,
                metadata,
                args.version,
                stateful_egress,
                rules,
            )
            .await?;

        if is_json {
            println!(
                "{}",
                serde_json::to_string_pretty(&nsg).map_err(CarbideCliError::JsonError)?
            );
        } else {
            convert_nsgs_to_table(&[nsg], true)?.printstd();
        }

        Ok(())
    }
}

impl Run for Args {
    async fn run(self, ctx: &mut RuntimeContext) -> CarbideCliResult<()> {
        cmd::update(self, ctx.config.format, &ctx.api_client).await
    }
}
