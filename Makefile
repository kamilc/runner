define V3EXT
subjectAltName = DNS:localhost
endef

export V3EXT

build:
	cargo build

test: certificates
	cargo test

example/v3.ext:
	echo "$$V3EXT" > example/v3.ext

example/ca.p8:
	openssl genpkey \
    -algorithm RSA \
        -pkeyopt rsa_keygen_bits:2048 \
        -pkeyopt rsa_keygen_pubexp:65537 | \
  openssl pkcs8 -topk8 -nocrypt > example/ca.p8

example/ca.other.p8:
	openssl genpkey \
    -algorithm RSA \
        -pkeyopt rsa_keygen_bits:2048 \
        -pkeyopt rsa_keygen_pubexp:65537 | \
  openssl pkcs8 -topk8 -nocrypt > example/ca.other.p8

example/ca.pem: example/ca.p8
	openssl req -x509 -new -nodes -key example/ca.p8 -sha512 -days 1825 -subj "/O=goteleport/CN=goteleport" -out example/ca.pem

example/ca.other.pem: example/ca.other.p8
	openssl req -x509 -new -nodes -key example/ca.other.p8 -sha512 -days 1825 -subj "/O=goteleport/CN=goteleport" -out example/ca.other.pem

example/server.p8:
	openssl genpkey \
    -algorithm RSA \
        -pkeyopt rsa_keygen_bits:2048 \
        -pkeyopt rsa_keygen_pubexp:65537 | \
  openssl pkcs8 -topk8 -nocrypt > example/server.p8

example/client.p8:
	openssl genpkey \
    -algorithm RSA \
        -pkeyopt rsa_keygen_bits:2048 \
        -pkeyopt rsa_keygen_pubexp:65537 | \
  openssl pkcs8 -topk8 -nocrypt > example/client.p8

example/server.in.pem: example/server.p8
	openssl req -new -key example/server.p8 -sha512 -subj "/O=goteleport/CN=server" -out example/server.in.pem

example/client.in.pem: example/client.p8
	openssl req -new -key example/client.p8 -sha512 -subj "/O=goteleport/CN=client" -out example/client.in.pem

example/server.pem: example/v3.ext example/server.in.pem example/ca.pem example/ca.p8 example/ca.pem
	openssl x509 -req -extfile example/v3.ext -sha512 -days 1825 -in example/server.in.pem -CA example/ca.pem -CAkey example/ca.p8 -CAcreateserial -out example/server.pem

example/client.pem: example/v3.ext example/client.in.pem example/ca.pem example/ca.p8 example/ca.pem
	openssl x509 -req -extfile example/v3.ext -sha512 -days 1825 -in example/client.in.pem -CA example/ca.pem -CAkey example/ca.p8 -CAcreateserial -out example/client.pem

clean-certificates:
	rm -f example/*.{p8,slr,pem,ext} example/verify

example/verify: example/server.pem example/client.pem example/ca.other.pem
	openssl verify -verbose -CAfile example/ca.pem example/client.pem &&\
	openssl verify -verbose -CAfile example/ca.pem example/server.pem &&\
	echo "OK" > example/verify

certificates: example/verify
