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
        #[clap(help = "The CSV file to use to update metadata for multiple skus")]
        pub filename: String,
    }
}

pub mod cmd {
    use ::rpc::admin_cli::CarbideCliError;

    use super::args::Args;
    use super::*;
    use crate::rpc::ApiClient;
    use crate::sku::update_metadata;

    pub async fn bulk_update_metadata(args: Args, api_client: &ApiClient) -> CarbideCliResult<()> {
        let mut rdr = csv::Reader::from_path(&args.filename)
            .map_err(|e| CarbideCliError::IOError(e.into()))?;

        // disable reading the first row as a header
        rdr.set_headers(vec!["sku id", "device type"].into());

        let mut current_line = 1;
        for result in rdr.records() {
            match result {
                Err(e) => {
                    // log and ignore parsing errors on a single line.
                    tracing::error!(
                        "Error reading file {} line {current_line}: {e}",
                        args.filename
                    );
                }
                Ok(data) => {
                    // Log missing SKUs, but don't stop processing
                    let Some(sku_id) = data.get(0).map(str::to_owned) else {
                        tracing::error!("No SKU ID at line {current_line}");
                        continue;
                    };
                    let device_type = data.get(1).filter(|s| !s.is_empty()).map(str::to_owned);
                    let description = data.get(2).filter(|s| !s.is_empty()).map(str::to_owned);

                    // log errors but don't stop the processing
                    if let Err(e) = api_client
                        .0
                        .update_sku_metadata(update_metadata::Args {
                            sku_id,
                            description,
                            device_type,
                        })
                        .await
                    {
                        tracing::error!("{e}");
                    }
                }
            }
            current_line += 1;
        }
        Ok(())
    }
}

impl Run for Args {
    async fn run(self, ctx: &mut RuntimeContext) -> CarbideCliResult<()> {
        cmd::bulk_update_metadata(self, &ctx.api_client).await
    }
}
