_default:
	@just --list --unsorted --list-heading '' --list-prefix '—— '

show output:
	fdp -T svg -O {{output}}
	$BROWSER {{output}}.svg
