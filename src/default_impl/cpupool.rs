
use futures::Future;
use utils::SendBoxFuture;

use futures_cpupool::{ CpuPool, Builder};

use context::RunElsewhere;

pub fn simple_cpu_pool() -> CpuPool {
    Builder::new().create()
}

impl RunElsewhere for CpuPool {
    /// executes the futures `fut` "elswhere" e.g. in a cpu pool
    fn execute<F>( &self, fut: F) -> SendBoxFuture<F::Item, F::Error>
        where F: Future + Send + 'static,
              F::Item: Send+'static,
              F::Error: Send+'static
    {
        Box::new( self.spawn( fut ) )
    }
}


#[cfg(test)]
mod test {
    use futures_cpupool::Builder;
    use core::error::*;
    use super::*;

    #[test]
    fn check_if_it_works() {
        let pool = Builder::new().create();
        _check_if_it_works( pool )
    }

    fn _check_if_it_works<R: RunElsewhere>(r: R) {
        let res = r.execute_fn( ||-> Result<u32> { Ok( 33u32 ) } ).wait();
        let val = assert_ok!( res );
        assert_eq!( 33u32, val );
    }
}