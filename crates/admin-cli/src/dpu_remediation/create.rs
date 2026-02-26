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

    #[derive(Parser, Debug)]
    pub struct Args {
        #[clap(help = "The filename of the script to run", long)]
        pub script_filename: String,
        #[clap(
            help = "specify the amount of retries for the remediation, defaults to no retries",
            long
        )]
        pub retries: Option<u32>,
        #[clap(
            long = "meta-name",
            value_name = "META_NAME",
            help = "The name that should be used as part of the Metadata for newly created Remediations.  Completely optional."
        )]
        pub meta_name: Option<String>,

        #[clap(
            long = "meta-description",
            value_name = "META_DESCRIPTION",
            help = "The description that should be used as part of the Metadata for newly created Remediations.  Completely optional."
        )]
        pub meta_description: Option<String>,

        #[clap(
            long = "label",
            value_name = "LABEL",
            help = "A label that will be added as metadata for the newly created Remediation. The labels key and value must be separated by a : character. E.g. DATACENTER:XYZ.  Completely optional.",
            action = clap::ArgAction::Append
        )]
        pub labels: Option<Vec<String>>,
    }

    impl Args {
        pub fn into_metadata(self) -> Option<::rpc::forge::Metadata> {
            if self.labels.is_none() && self.meta_name.is_none() && self.meta_description.is_none()
            {
                return None;
            }

            let mut labels = Vec::new();
            if let Some(list) = &self.labels {
                for label in list {
                    let label = match label.split_once(':') {
                        Some((k, v)) => rpc::forge::Label {
                            key: k.trim().to_string(),
                            value: Some(v.trim().to_string()),
                        },
                        None => rpc::forge::Label {
                            key: label.trim().to_string(),
                            value: None,
                        },
                    };
                    labels.push(label);
                }
            }

            Some(::rpc::forge::Metadata {
                name: self.meta_name.unwrap_or_default(),
                description: self.meta_description.unwrap_or_default(),
                labels,
            })
        }
    }
}

pub mod cmd {
    use ::rpc::admin_cli::CarbideCliError;
    use rpc::forge::CreateRemediationRequest;

    use super::args::Args;
    use crate::rpc::ApiClient;

    pub async fn create_dpu_remediation(
        create_remediation: Args,
        api_client: &ApiClient,
    ) -> Result<(), CarbideCliError> {
        let script = tokio::fs::read_to_string(&create_remediation.script_filename)
            .await
            .map_err(|err| {
                tracing::error!("Error reading script file for dpu remediation: {:?}", err);
                CarbideCliError::IOError(err)
            })?;

        let response = api_client
            .0
            .create_remediation(CreateRemediationRequest {
                script,
                retries: create_remediation.retries.unwrap_or_default() as i32,
                metadata: create_remediation.into_metadata(),
            })
            .await?;

        tracing::info!("Created remediation with id: {:?}", response.remediation_id);
        Ok(())
    }
}

impl Run for Args {
    async fn run(self, ctx: &mut RuntimeContext) -> CarbideCliResult<()> {
        cmd::create_dpu_remediation(self, &ctx.api_client).await
    }
}
