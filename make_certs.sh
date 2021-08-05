#!/bin/bash
set -ex

mkdir -p certs/CA

ROOT_CA_KEY_PATH="certs/CA/root_ca_key.pem"
ROOT_CA_CRT_PATH="certs/CA/root_ca_crt.pem"

SERVER_CA_KEY_PATH="certs/CA/server_ca_key.pem"
SERVER_CA_CRS_PATH="certs/CA/server_ca.csr"
SERVER_CA_CRT_PATH="certs/CA/server_ca_crt.pem"

SERVER1_KEY_PATH="certs/server1_ca_key.pem"
SERVER1_CRS_PATH="certs/server1_ca.csr"
SERVER1_CRT_PATH="certs/server1_ca_crt.pem"

CLIENT_CA_KEY_PATH="certs/CA/client_ca_key.pem"
CLIENT_CA_CRS_PATH="certs/CA/client_ca.csr"
CLIENT_CA_CRT_PATH="certs/CA/client_ca_crt.pem"


CLIENT1_KEY_PATH="certs/client1_ca_key.pem"
CLIENT1_CRS_PATH="certs/client1_ca.csr"
CLIENT1_CRT_PATH="certs/client1_ca_crt.pem"

CLIENT2_KEY_PATH="certs/client2_ca_key.pem"
CLIENT2_CRS_PATH="certs/client2_ca.csr"
CLIENT2_CRT_PATH="certs/client2_ca_crt.pem"

UNAUTH_KEY_PATH="certs/unsigned_key.pem"
UNAUTH_CRT_PATH="certs/unsigned_key.crt"



#generate our private key and self-signed certificate for the root CA
openssl req -x509 -newkey ec:<(openssl ecparam -name prime256v1) -keyout $ROOT_CA_KEY_PATH -out $ROOT_CA_CRT_PATH -days 356 -nodes -subj '/CN=Root Cert Authority'

#generate server CA private key and cert sign request (CSR) 
openssl req -new -newkey ec:<(openssl ecparam -name prime256v1) -keyout $SERVER_CA_KEY_PATH -out $SERVER_CA_CRS_PATH -days 356 -nodes -subj '/CN=Server Cert Authority'
#sign server CA CRS with root CA
openssl x509 -req -days 365 -in $SERVER_CA_CRS_PATH -CA $ROOT_CA_CRT_PATH -CAkey $ROOT_CA_KEY_PATH -set_serial 01 -out $SERVER_CA_CRT_PATH


#generate server1 private key and cert sign request (CSR) 
openssl req -new -newkey ec:<(openssl ecparam -name prime256v1) -keyout $SERVER1_KEY_PATH -out $SERVER1_CRS_PATH -days 356 -nodes -subj '/CN=server1'
#sign server1 CRS with server CA
openssl x509 -req -days 365 -in $SERVER1_CRS_PATH -CA $SERVER_CA_CRT_PATH -CAkey $SERVER_CA_KEY_PATH -set_serial 01 -out $SERVER1_CRT_PATH


#generate client CA private key and cert sign request (CSR) 
openssl req -new -newkey ec:<(openssl ecparam -name prime256v1) -keyout $CLIENT_CA_KEY_PATH -out $CLIENT_CA_CRS_PATH -days 356 -nodes -subj '/CN=Server Cert Authority'
#sign client CA CRS with root CA
openssl x509 -req -days 365 -in $CLIENT_CA_CRS_PATH -CA $ROOT_CA_CRT_PATH -CAkey $ROOT_CA_KEY_PATH -set_serial 01 -out $CLIENT_CA_CRT_PATH



#generate client1 private key and cert sign request (CSR) 
openssl req -new -newkey ec:<(openssl ecparam -name prime256v1) -keyout $CLIENT1_KEY_PATH -out $CLIENT1_CRS_PATH -days 356 -nodes -subj '/CN=client1'
#sign client1 CRS with server CA
openssl x509 -req -days 365 -in $CLIENT1_CRS_PATH -CA $CLIENT_CA_CRT_PATH -CAkey $CLIENT_CA_KEY_PATH -set_serial 01 -out $CLIENT1_CRT_PATH
#generate client2 private key and cert sign request (CSR) 
openssl req -new -newkey ec:<(openssl ecparam -name prime256v1) -keyout $CLIENT2_KEY_PATH -out $CLIENT2_CRS_PATH -days 356 -nodes -subj '/CN=client2'
#sign client2 CRS with server CA
openssl x509 -req -days 365 -in $CLIENT2_CRS_PATH -CA $CLIENT_CA_CRT_PATH -CAkey $CLIENT_CA_KEY_PATH -set_serial 02 -out $CLIENT2_CRT_PATH

#generate a selfsigned but unauthorized cert that attemps to immitate client1
openssl req -x509 -newkey ec:<(openssl ecparam -name prime256v1) -keyout $UNAUTH_KEY_PATH -out $UNAUTH_CRT_PATH -days 356 -nodes -subj '/CN=client1'

#cleanup certificate sign requests
rm -rf certs/*.csr certs/CA/*.csr