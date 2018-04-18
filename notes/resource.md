
                 can be merged but then we would have to use unsafe sync
                  __|________________________________
                 |                                   |
	             
new---> Source => Mutex<Future> => RwLock<TFileBuffer> -->-\
          |                        /                         |
           \-----<----------------/--------<---------------/
	                             /
			|                   / |                  |
			|		    ______ /   \__________________/
		This could be and		    |
		alternative path		Could be Arc<TFileBuffer> if there would not
		using cache lockup		be an unload feature (pointer cmpexc?)
        we already need a
		context so why not
		use `ctx.cached(Source)`
		and `ctx.cache_update(Source)`
