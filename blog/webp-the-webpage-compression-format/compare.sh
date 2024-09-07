#!/usr/bin/env bash
cd corpus

printf "%24s%8s%8s%8s%8s%8s\n" File Raw gzip brotli bzip2 webp
for file in *; do
	printf \
		"%24s%8d%8d%8d%8d%8d\n" \
		"$file" \
		$(<"$file" wc -c) \
		$(gzip --best <"$file" | wc -c) \
		$(brotli --best <"$file" | wc -c) \
		$(bzip2 --best <"$file" | wc -c) \
		$(../compressor/target/release/compressor <"$file" | wc -c)
done
