#!/bin/bash
set -ex

mkdir -p certs/test

openssl req -x509 -newkey ec:<(openssl ecparam -name prime256v1) -keyout certs/test/good_key.pem -out certs/test/good_crt.pem -days 3650 -nodes -subj '/CN=client1/O=client'
openssl req -x509 -newkey ec:<(openssl ecparam -name prime256v1) -keyout certs/test/invalid_org_name_key.pem -out certs/test/invalid_org_name_crt.pem -days 3650 -nodes -subj '/CN=client1/O=invalid'
openssl req -x509 -newkey ec:<(openssl ecparam -name prime256v1) -keyout certs/test/missing_org_name_key.pem -out certs/test/missing_org_name_crt.pem -days 3650 -nodes -subj '/CN=client1'
openssl req -x509 -newkey ec:<(openssl ecparam -name prime256v1) -keyout certs/test/missing_cn_key.pem -out certs/test/missing_cn_crt.pem -days 3650 -nodes -subj '/'