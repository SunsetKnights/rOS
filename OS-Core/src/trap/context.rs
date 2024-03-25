use riscv::register::sstatus::{self, Sstatus, SPP};

// It is necessary to ensure that the memory layout of this structure is consistent with that in trap.S
#[repr(C)] // this unions means, use C struct memory layout
#[derive(Clone, Copy)]
pub struct TrapContext {
    // general purpose register, x[0] is offset 0, x[1] is offset 8...
    pub x: [usize; 32],
    // sstatus register
    pub sstatus: Sstatus,
    // sepc register, trap return address
    pub sepc: usize,
    // kernel root ppn
    pub kernel_satp: usize,
    // application's kernel stack address
    pub kernel_sp: usize,
    // trap handler
    pub trap_handler: usize,
}

impl TrapContext {
    /// Set user stack pointer.
    pub fn set_sp(&mut self, sp: usize) {
        self.x[2] = sp;
    }

    /// Init a user app context
    pub fn init_app_context(
        entry: usize,
        sp: usize,
        kernel_satp: usize,
        kernel_sp: usize,
        trap_handler: usize,
    ) -> Self {
        let mut sstatus = sstatus::read();
        sstatus.set_spp(SPP::User);
        let mut ret = Self {
            x: [0; 32],
            sstatus,
            sepc: entry,
            kernel_satp,
            kernel_sp,
            trap_handler,
        };
        ret.set_sp(sp);
        ret
    }
}
