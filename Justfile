_default:
	@just --list --unsorted --list-heading '' --list-prefix '—— '

run *args:
	cargo run {{args}}
	fdp -T svg -O output.dot
	# $BROWSER output.dot.png
	zen output.dot.svg
