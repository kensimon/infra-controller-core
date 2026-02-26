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
        #[clap(
            short = 'i',
            long,
            help = "Optional, unique ID to use when creating the instance type"
        )]
        pub id: Option<String>,

        #[clap(short = 'n', long, help = "Name of the instance type")]
        pub name: Option<String>,

        #[clap(short = 'd', long, help = "Description of the instance type")]
        pub description: Option<String>,

        #[clap(
            short = 'l',
            long,
            help = "JSON map of simple key:value pairs to be applied as labels to the instance type"
        )]
        pub labels: Option<String>,

        #[clap(
            short = 'f',
            long,
            help = "Optional, JSON array containing a set of instance type capability filters"
        )]
        pub desired_capabilities: Option<String>,
    }
}

pub mod cmd {
    use ::rpc::admin_cli::{CarbideCliError, OutputFormat};
    use ::rpc::forge::{
        CreateInstanceTypeRequest, InstanceTypeAttributes, {self as forgerpc},
    };

    use super::args::Args;
    use super::*;
    use crate::instance_type::common::convert_itypes_to_table;
    use crate::rpc::ApiClient;

    /// Create an instance type.
    /// On successful creation, the details of the
    /// new type will be displayed.
    pub async fn create(
        args: Args,
        output_format: OutputFormat,
        api_client: &ApiClient,
    ) -> CarbideCliResult<()> {
        let is_json = output_format == OutputFormat::Json;

        let id = args.id;

        let labels = if let Some(l) = args.labels {
            serde_json::from_str(&l)?
        } else {
            vec![]
        };

        let metadata = forgerpc::Metadata {
            name: args.name.unwrap_or_default(),
            description: args.description.unwrap_or_default(),
            labels,
        };

        let instance_type_attributes = args
            .desired_capabilities
            .map(|d| {
                serde_json::from_str(&d).map(|desired_capabilities| InstanceTypeAttributes {
                    desired_capabilities,
                })
            })
            .transpose()?;

        let itype = api_client
            .0
            .create_instance_type(CreateInstanceTypeRequest {
                id,
                metadata: Some(metadata),
                instance_type_attributes,
            })
            .await?
            .instance_type
            .ok_or(CarbideCliError::Empty)?;

        if is_json {
            println!(
                "{}",
                serde_json::to_string_pretty(&itype).map_err(CarbideCliError::JsonError)?
            );
        } else {
            convert_itypes_to_table(&[itype], true)?.printstd();
        }

        Ok(())
    }
}

impl Run for Args {
    async fn run(self, ctx: &mut RuntimeContext) -> CarbideCliResult<()> {
        cmd::create(self, ctx.config.format, &ctx.api_client).await
    }
}
