use std::sync::{
    Mutex,
    Arc
};

use std::collections::{
    VecDeque,
    HashMap
};
use std::pin::Pin;
use std::future::Future;
use std::os::unix::io::RawFd;

use libc::{
    EPOLL_CTL_ADD, EPOLLIN, EPOLLOUT, epoll_event, epoll_wait
};

use std::task::{
    Waker, 
    Wake, 
    Context, 
    Poll
};

pub struct Executor {
    pub queue: Mutex<VecDeque<Arc<Task>>>, // VecDeque - швидше за звичайний Vector і парцює з push_front, push_back. Що є доволі зручним у контексті мого рантайму (vec - 6 O(n), VecDeque O(1)), вона надає FIFO, тому це зручно.
}
pub struct Task {
    future: Mutex<Pin<Box<dyn Future<Output = ()> + Send>>>, // Pin гарантує, що після запінення future не буде переміщена.
    executor: Arc<Executor> // Таска сама мусить себе заплоновуват через Waker, а це вимагає володіння Executor, а не посилання.
}

pub struct Reactor {
    epoll_fd: RawFd,
    wakers: Mutex<HashMap<RawFd, Waker>>
}

impl Executor {
    pub fn spawn<F>(self: &Arc<Self>, future: F)
    where
        F: Future<Output = ()> + Send + 'static
    {
        let task = Arc::new(Task {
            future: Mutex::new(Box::pin(future)),
            executor: self.clone(),
        });

        // ставимо таску на виконання в чергу
        self.queue.lock().unwrap().push_back(task);
    }

    pub fn run(&self, reactor: &Reactor) {
        while let Some(task) = self.queue.lock().unwrap().pop_front() {
            let waker = Waker::from(task.clone());
            let mut cx = Context::from_waker(&waker);

            let mut future = task.future.lock().unwrap();

            if let Poll::Pending = future.as_mut().poll(&mut cx) {};
        }

        reactor.wait();
    }
}

impl Wake for Task {
    fn wake(self: Arc<Self>) {
        let exec = self.executor.clone();
        let mut queue = exec.queue.lock().unwrap();
        queue.push_back(self);
    }
}

impl Reactor {
    pub fn new() -> Self {
        let epoll_fd = unsafe {
            libc::epoll_create(0)
        };
        
        Self {
            epoll_fd,
            wakers: Mutex::new(HashMap::new()),
        }
    }

    pub fn register(&self, fd: RawFd, writable: bool, waker: Waker) {
        let mut event = epoll_event {
            events: if writable { EPOLLOUT as u32 } else { EPOLLIN as u32 },
            u64: fd as u64, 
        };
        unsafe {
            libc::epoll_ctl(self.epoll_fd, EPOLL_CTL_ADD, fd, &mut event as *mut _,);
        }

        self.wakers.lock().unwrap().insert(fd, waker);
    }

    pub fn wait(&self) {
        let mut events = [libc::epoll_event { events: 0, u64: 0} ];
        let nfds = unsafe {
            epoll_wait(self.epoll_fd, events.as_mut_ptr(), 1024, -1)
        };

        for i in 0..nfds as usize {
            let fd = events[i].u64 as RawFd;
            if let Some(waker)  = self.wakers.lock().unwrap().remove(&fd) {
                waker.wake();
            }
        }
    }
}