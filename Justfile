_default:
	@just --list --unsorted --list-heading '' --list-prefix '—— '

show output:
	fdp -T svg -O {{output}}
	$BROWSER {{output}}.svg

build-web:
	typst compile --features bundle,html --format bundle web/main.typ public
serve-web:
	typst watch --features bundle,html --format bundle web/main.typ public

clean:
	rm -rd .cache/queries
	rm -rd output
