all:
	cargo build
run:
	cargo run
push:
	jj bookmark set main -r @-
	jj git push --bookmark main

