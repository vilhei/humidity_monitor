#![no_std]
#![no_main]

use esp_alloc as _;
use esp_backtrace as _;
use esp_hal::{
    clock::CpuClock,
    delay::Delay,
    entry,
    gpio::Output,
    i2c::master::I2c,
    peripherals::LPWR,
    rng::Rng,
    rtc_cntl::{sleep::TimerWakeupSource, Rtc},
    timer::timg::TimerGroup,
};
use esp_wifi::esp_now::PeerInfo;
use fugit::RateExtU32;

use core::time::Duration;

const CENTRAL_NODE_MAC_ADDRESS: [u8; 6] = [72, 202, 67, 207, 242, 60];
const CENTRAL_NODE2_MAC_ADDRESS: [u8; 6] = [0x54, 0x32, 0x04, 0x47, 0xd2, 0x9c];

const PERIOD_TIME: Duration = Duration::from_secs(60);

#[entry]
fn main() -> ! {
    let mut config = esp_hal::Config::default();
    config.cpu_clock = CpuClock::Clock160MHz;
    let peripherals = esp_hal::init(config);
    measure(peripherals);
}

fn measure(peripherals: esp_hal::peripherals::Peripherals) -> ! {
    let mut led = Output::new(peripherals.GPIO0, esp_hal::gpio::Level::Low);
    led.set_high();
    let i2c_dev = I2c::new(
        peripherals.I2C0,
        esp_hal::i2c::master::Config {
            frequency: 200u32.kHz(),
            ..Default::default()
        },
    )
    .with_sda(peripherals.GPIO3)
    .with_scl(peripherals.GPIO4);

    let mut aht = aht10_embedded::AHT10::new(i2c_dev);
    aht.initialize().unwrap();
    let timg0 = TimerGroup::new(peripherals.TIMG0);

    esp_alloc::heap_allocator!(72 * 1024);

    let wifi_controller = esp_wifi::init(
        timg0.timer0,
        Rng::new(peripherals.RNG),
        peripherals.RADIO_CLK,
    )
    .expect("Failed to initialize wifi");

    let mut esp_now = esp_wifi::esp_now::EspNow::new(&wifi_controller, peripherals.WIFI)
        .expect("Failed to create esp now instance");

    add_esp_now_peer(&esp_now, CENTRAL_NODE_MAC_ADDRESS);
    add_esp_now_peer(&esp_now, CENTRAL_NODE2_MAC_ADDRESS);

    let mut delay = Delay::new();

    match aht.read_data(&mut delay) {
        Ok(data) => {
            let tmp = data.temperature_celsius();
            let hum = data.humidity();
            let msg = [&tmp.to_ne_bytes()[..], &hum.to_ne_bytes()[..]].concat();
            send_sensor_msg(&mut esp_now, &msg, &CENTRAL_NODE_MAC_ADDRESS);
            send_sensor_msg(&mut esp_now, &msg, &CENTRAL_NODE2_MAC_ADDRESS);
        }
        Err(_e) => {}
    }

    drop(esp_now);
    wifi_controller
        .deinit()
        .expect("Failed to deinit wifi controller");

    enter_deep_sleep(peripherals.LPWR, PERIOD_TIME);
}

fn send_sensor_msg(esp_now: &mut esp_wifi::esp_now::EspNow<'_>, msg: &[u8], addr: &[u8; 6]) {
    let send_waiter = esp_now
        .send(addr, msg)
        .expect("failed to send esp now message")
        .wait();

    match send_waiter {
        Ok(_) => {}
        Err(e) => match e {
            esp_wifi::esp_now::EspNowError::Error(_) => todo!(),
            esp_wifi::esp_now::EspNowError::SendFailed => (),
            esp_wifi::esp_now::EspNowError::DuplicateInstance => todo!(),
            esp_wifi::esp_now::EspNowError::Initialization(_) => todo!(),
        },
    }
}

fn add_esp_now_peer(esp_now: &esp_wifi::esp_now::EspNow<'_>, peer_address: [u8; 6]) {
    esp_now
        .add_peer(PeerInfo {
            peer_address,
            lmk: None,
            channel: None,
            encrypt: false,
        })
        .expect("Failed to add central node peer info");
}

fn enter_deep_sleep(lpwr: LPWR, duration: Duration) -> ! {
    let wakeup_source = TimerWakeupSource::new(duration);
    let mut rtc = Rtc::new(lpwr);
    rtc.sleep_deep(&[&wakeup_source]);
}
