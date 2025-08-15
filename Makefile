all:
	cargo build
run:
	cargo run
push:
	jj bookmark create main -r @-
	jj git push -c @-

