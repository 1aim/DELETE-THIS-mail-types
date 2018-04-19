
use futures::Future;
use utils::SendBoxFuture;

use futures_cpupool::{ CpuPool, Builder};

use context::OffloaderComponent;

pub fn simple_cpu_pool() -> CpuPool {
    Builder::new().create()
}

impl OffloaderComponent for CpuPool {
    /// executes the futures `fut` "elswhere" e.g. in a cpu pool
    fn offload<F>(&self, fut: F) -> SendBoxFuture<F::Item, F::Error>
        where F: Future + Send + 'static,
              F::Item: Send+'static,
              F::Error: Send+'static
    {
        Box::new( self.spawn( fut ) )
    }
}


#[cfg(test)]
mod test {
    use futures::future;
    use futures_cpupool::Builder;
    use super::*;

    #[test]
    fn check_if_it_works() {
        let pool = Builder::new().create();
        _check_if_it_works( pool )
    }

    fn _check_if_it_works<R: OffloaderComponent>(r: R) {
        let res = r.offload(future::lazy(||-> Result<u32,  ()> { Ok(33u32) } )).wait();
        let val = assert_ok!( res );
        assert_eq!( 33u32, val );
    }
}