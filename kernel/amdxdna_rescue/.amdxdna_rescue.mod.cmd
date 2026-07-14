savedcmd_amdxdna_rescue.mod := printf '%s\n'   amdxdna_rescue.o | awk '!x[$$0]++ { print("./"$$0) }' > amdxdna_rescue.mod
