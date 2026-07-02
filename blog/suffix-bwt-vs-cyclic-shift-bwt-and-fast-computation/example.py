s = "bcacaba"
n = len(s)

shifts = sorted(range(n), key = lambda i: s[i:] + s[:i])
suffixes = sorted(range(n), key = lambda i: s[i:])

print("last char of first line non-unique", s.count(s[shifts[0] - 1]) > 1)
print("distinct", shifts != suffixes)
print("not last suffix", suffixes[-1] != 0)
print()

for i in shifts:
	print(i, s[i - 1], s[i:] + s[:i])
print(*(s[i - 1] for i in shifts), sep="")
print()

for i in suffixes:
	print(i, s[i - 1], s[i:])
print(*(s[i - 1] for i in suffixes), sep="")
