
# current load + encode algo

1. start loading resources
2. wait for it to be done
3. start encoding everything into parts
4. combine thinks

# new algo

1. start loading resources
2. wait for it to be done (we could start encoding, but well it troublesome
   wrt. e.g. transfer encoding header field etc.)
3. start encoding writing everything into _one buffer_
   (we already have loaded the bodies, so why not)
