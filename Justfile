_default:
	@just --list --unsorted --list-heading '' --list-prefix '—— '

show output:
	fdp -T svg -O {{output}}
	# $BROWSER output.dot.png
	zen {{output}}.svg
