all:
	cargo build
run:
	cargo run
push:
	jj git push --allow-new -c @-
	jj bookmark set main -r @-
	jj git push --bookmark main

