#![no_main]
#![no_std]

use panic_halt as _;

use crate::hal::spi::{Mode, Phase, Polarity};
use crate::hal::{gpio::Pull, prelude::*};
use cortex_m_rt::entry;
use stm32f4xx_hal as hal;
use stm32f4xx_hal::{
    pac::{self, TIM4},
    prelude::*,
    timer::{Channel, ChannelBuilder},
};

pub const MODE: Mode = Mode {
    phase: Phase::CaptureOnFirstTransition,
    polarity: Polarity::IdleLow,
};

#[entry]
fn main() -> ! {
    let dp = pac::Peripherals::take().unwrap();
    let rcc = dp.RCC.constrain();
    let clocks = rcc.cfgr.sysclk(84.MHz()).freeze();

    let gpiob = dp.GPIOB.split();
    let gpioa = dp.GPIOA.split();

    let pwm_pin = gpiob.pb8.into_alternate(); // ШИМ на PB8

    // Настройка SPI
    let sck = gpioa.pa5.internal_resistor(Pull::Up);
    let miso = gpioa.pa6.internal_resistor(Pull::Down);
    let mosi = gpioa.pa7.internal_resistor(Pull::None);

    let mut spi = dp.SPI1.spi((sck, miso, mosi), MODE, 1.MHz(), &clocks);

    // Создаём канал ШИМ
    let chan: ChannelBuilder<TIM4, 2, false, _> = ChannelBuilder::new(pwm_pin);
    let mut pwm = dp.TIM4.pwm_hz(chan, 20.kHz(), &clocks);

    let mut cs = gpioa.pa4.into_push_pull_output();
    cs.set_high();

    let mut led = gpioa.pa0.into_push_pull_output();
    led.set_low();

    let vref = 5.0; // Опорное напряжение
    let adc_max_value: f32 = 4095.0;
    const TEMPERATURE_THRESHOLD: f32 = 50.0;

    let calculated_threshold = TEMPERATURE_THRESHOLD * adc_max_value / (vref * 100.0);
    let max_duty = pwm.get_max_duty();

    pwm.set_duty(Channel::C2, max_duty);
    pwm.enable(Channel::C2);

    let mut buffer = [0x00, 0x00];

    loop {
        cs.set_low();
        spi.transfer_in_place(&mut buffer).unwrap();
        cs.set_high();

        let raw_value: u16 = ((buffer[0] as u16) << 8 | (buffer[1] as u16)) >> 2 & 0x0FFF; // Mask 12 bits

        // Управление светодиодом
        if raw_value as f32 > calculated_threshold {
            led.set_high();
        } else {
            led.set_low();
        }

        // Пропорциональное управление вентилятором
        let duty_cycle = ((raw_value as f32 / adc_max_value) * max_duty as f32) as u16;
        pwm.set_duty(Channel::C2, duty_cycle);
    }
}
