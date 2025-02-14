#!/bin/env bash
cd entity/src
sea generate entity --with-serde serialize -s auth -o auth
sea generate entity --with-serde serialize -s public -o public

sed -i 's/super::users/crate::auth::users/g' public/*.rs

cd ../..
