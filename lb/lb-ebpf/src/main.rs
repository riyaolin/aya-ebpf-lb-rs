#![no_std]
#![no_main]

use aya_bpf::{bindings::xdp_action, helpers::bpf_csum_diff, macros::xdp, programs::XdpContext};
use aya_log_ebpf::info;
use aya_bpf::helpers::bpf_printk;

use core::mem;
use network_types::{
    eth::{EthHdr, EtherType},
    ip::{IpProto, Ipv4Hdr},
    tcp::TcpHdr,
    udp::UdpHdr,
};

// constants for demo
const CLIENT: u32 = 0xAC130003;
const CLIENT_E: u8 = 3;
const LB: u32 = 0xAC130005;
const LB_E: u8 = 5;
const BACKEND_A: u32 = 0xAC130002;
const BACKEND_A_E: u8 = 2;
const _BACKEND_B: u32 = 0xAC130004;

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    unsafe { core::hint::unreachable_unchecked() }
}

#[xdp]
pub fn xdp_lb(ctx: XdpContext) -> u32 {
    info!(&ctx, "New packet in...");
    match try_xdp_lb(ctx) {
        Ok(ret) => ret,
        Err(_) => xdp_action::XDP_ABORTED,
    }
}

#[inline(always)]
fn ptr_at_mut<T>(ctx: &XdpContext, offset: usize) -> Result<*mut T, ()> {
    let ptr = ptr_at::<T>(ctx, offset)?;
    Ok(ptr as *mut T)
}

#[inline(always)]
fn ptr_at<T>(ctx: &XdpContext, offset: usize) -> Result<*const T, ()> {
    let start = ctx.data();
    let end = ctx.data_end();
    let len = mem::size_of::<T>();

    if start + offset + len > end {
        return Err(());
    }

    Ok((start + offset) as *const T)
}

// Converts a checksum into u16
#[inline(always)]
pub fn csum_fold_helper(mut csum: u64) -> u16 {
    for _i in 0..4 {
        if (csum >> 16) > 0 {
            csum = (csum & 0xffff) + (csum >> 16);
        }
    }
    return !(csum as u16);
}

fn try_xdp_lb(ctx: XdpContext) -> Result<u32, ()> {
    let ethhdr: *mut EthHdr = ptr_at_mut(&ctx, 0)?; // (2)
    match unsafe { (*ethhdr).ether_type } {
        EtherType::Ipv4 => {}
        _ => return Ok(xdp_action::XDP_PASS),
    }

    let ipv4hdr: *mut Ipv4Hdr = ptr_at_mut(&ctx, EthHdr::LEN)?;
    let source_addr = u32::from_be(unsafe { (*ipv4hdr).src_addr });
    let _dst_addr = u32::from_be(unsafe { (*ipv4hdr).dst_addr });
    // debugging for checking little/big endian
    unsafe {
        info!(&ctx, "Destination addr: {:i}", (*ipv4hdr).dst_addr);
        info!(&ctx, "checksum: {}", (*ipv4hdr).check);
    }

    let source_port = match unsafe { (*ipv4hdr).proto } {
        IpProto::Tcp => {
            let tcphdr: *const TcpHdr =
                ptr_at(&ctx, EthHdr::LEN + Ipv4Hdr::LEN)?;
            u16::from_be(unsafe { (*tcphdr).source })
        }
        IpProto::Udp => {
            let udphdr: *const UdpHdr =
                ptr_at(&ctx, EthHdr::LEN + Ipv4Hdr::LEN)?;
            u16::from_be(unsafe { (*udphdr).source })
        }
        _ => return Err(()),
    };

    // debugging prints: one to terminal; the other to trace_pipe
    info!(&ctx, "SRC IP: {:i}, SRC PORT: {}", source_addr, source_port);
    // `bpf_printk!` calls unsafe function `bpf_printk_impl`
    unsafe {
        bpf_printk!(b"Source addr in hex: %X, port: %d", source_addr, source_port);
    }

    if source_addr == CLIENT {
        info!(&ctx, "From client");
        unsafe {
            // (*ipv4hdr).dst_addr = u32::from_be(BACKEND_A);
            (*ipv4hdr).dst_addr = BACKEND_A.to_be();
            (*ethhdr).dst_addr[5] = BACKEND_A_E;
            info!(&ctx, "Destination now changed to: {:i}", (*ipv4hdr).dst_addr);
        }
    } else if source_addr == BACKEND_A {
        info!(&ctx, "From backend host");
        unsafe {
            // (*ipv4hdr).dst_addr = u32::from_be(CLIENT);
            (*ipv4hdr).dst_addr = CLIENT.to_be();
            (*ethhdr).dst_addr[5] = CLIENT_E;
            info!(&ctx, "Destination now changed to: {:i}", (*ipv4hdr).dst_addr);
        }
    } else {
        info!(&ctx, "From unrelated hosts in the network");
        return Ok(xdp_action::XDP_PASS)
    }
    
    unsafe {
        (*ipv4hdr).src_addr = u32::from_be(LB);
        (*ethhdr).src_addr[5] = LB_E;
    }
    
    let full_cksum = unsafe {
        bpf_csum_diff(
            mem::MaybeUninit::zeroed().assume_init(),
            0,
            ipv4hdr as *mut u32,
            Ipv4Hdr::LEN as u32,
            0,
        )
    } as u64;

    unsafe { (*ipv4hdr).check = csum_fold_helper(full_cksum) };
    
    Ok(xdp_action::XDP_TX)
}
