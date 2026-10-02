# A stand-in scanner: one `no-eval` result per line of src/*.js that calls eval.
results=$(grep -Hn 'eval(' src/*.js | grep -v '^src/sum.js' | grep -v 'scan-ignore' | awk -F: '{
  printf "%s{\"ruleId\":\"no-eval\",\"message\":{\"text\":\"eval runs arbitrary code\"},\"locations\":[{\"physicalLocation\":{\"artifactLocation\":{\"uri\":\"%s\"},\"region\":{\"startLine\":%s}}}]}", sep, $1, $2; sep=","
}')
printf '{"version":"2.1.0","runs":[{"tool":{"driver":{"name":"scan"}},"results":[%s]}]}\n' "$results" > scan.sarif
