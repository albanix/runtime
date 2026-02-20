use std::sync::{
    Mutex,
    Arc
};
use std::collections::VecDeque;
use std::pin::Pin;
use std::future::Future;
use std::task::RawWaker;
struct Executor {
    queue: Mutex<VecDeque<Arc<Task>>>, // VecDeque - швидше за звичайний Vector і парцює з push_front, push_back. Що є доволі зручним у контексті мого рантайму (vec - 6 O(n), VecDeque O(1)), вона надає FIFO, тому це зручно.
}
struct Task {
    future: Mutex<Pin<Box<dyn Future<Output = ()> + Send>>>, // Pin гарантує, що після запінення future не буде переміщена.
    executor: Arc<Executor> // Таска сама мусить себе заплоновуват через Waker, а це вимагає володіння Executor, а не посилання.
}

impl Executor {
    fn spawn<F>(&self, x: &Arc<Self>, future: F)
    where
        F: Future<Output = ()> + Send + 'static
    {
        let task = Arc::new(Task {
            future: Mutex::new(Box::pin(future)),
            executor: x.clone(),
        });

        while let Some(task) = self.queue.lock().unwrap().pop_front() {
            //
        }
    }
}

fn waker() {

}

unsafe fn wake_by_ref(ptr: *const ()) {
    let arc = unsafe {
        Arc::<Task>::from_raw(ptr as *const Task)
    };
    arc.executor.queue.lock().unwrap().push_back(arc.clone());
    std::mem::forget(arc);
}

unsafe fn drop(ptr: *const()) {
    unsafe {
        Arc::from_raw(ptr as *const Task)
    };
}

fn clone(ptr: *const ()) -> RawWaker {
    let arc = unsafe { Arc::<Task>::from_raw(ptr as *const Task) }; // відновлюмо Arc<Task> який прилетів з into_raw: ! кожен конкретний poiter, який був отриманний з into_raw має бути повернено через from_raw = рівно один раз ! 
    let cloned = arc.clone();
    std::mem::forget(cloned);
    RawWaker::new(Arc::into_raw(cloned) as *const (), )
}
fn main()_{

}