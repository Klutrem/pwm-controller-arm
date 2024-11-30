#![no_main]
#![no_std]

use crate::hal::spi::{Mode, Phase, Polarity};
use crate::hal::{gpio::Pull, pac, prelude::*};
use cortex_m_rt::entry;
use panic_halt as _;
use stm32f4xx_hal as hal;
use stm32f4xx_hal::pac::TIM4;
use stm32f4xx_hal::timer::{Channel, ChannelBuilder};

/// Режим работы SPI для LTC1292
pub const MODE: Mode = Mode {
    phase: Phase::CaptureOnFirstTransition,
    polarity: Polarity::IdleLow,
};

#[entry]
fn main() -> ! {
    // Take device peripherals
    let p = pac::Peripherals::take().unwrap();

    // Настраиваем RCC (клоки и питание) и замораживаем конфигурацию
    let rcc = p.RCC.constrain();
    let clocks = rcc.cfgr.freeze();

    // Получаем GPIO порты
    let gpioa = p.GPIOA.split();
    let gpiob = p.GPIOB.split();

    // Конфигурируем пины для SPI
    let sck = gpioa.pa5.internal_resistor(Pull::Up);
    let miso = gpioa.pa6.internal_resistor(Pull::Down); // LTC1292 Шлёт сюда
    let mosi = gpioa.pa7.internal_resistor(Pull::None); // Для чтения не используется

    // Конфигурируем пин для выбора чипа (Chip Select, CS)
    let mut cs = gpioa.pa4.into_push_pull_output();
    cs.set_high(); // Делаем CS неактивным (высокий уровень), говорим, что мы ведущее устройство

    // Конфигурируем пин для светодиода
    let mut led = gpioa.pa0.into_push_pull_output();
    led.set_low(); // Turn LED off initially

    // Инициализируем SPI
    let mut spi = p.SPI1.spi((sck, miso, mosi), MODE, 1.MHz(), &clocks);

    let mut buffer = [0x00, 0x00]; // Буфер для приёма данных от SPI (2 байта)

    let vref = 5.0; // Опорное напряжение
    let adc_max_value: f32 = 4095.0; // Максимальное значение АЦП

    const TEMPERATURE_THRESHOLD: f32 = 50.0;
    const CRITICAL_TEMPERATURE: f32 = 130.0;

    let calculated_threshold = TEMPERATURE_THRESHOLD * adc_max_value / (vref * 100.0);
    let critical_threshold = CRITICAL_TEMPERATURE * adc_max_value / (vref * 100.0);

    // Настраиваем PWM на пине
    let pwm_pin = gpiob.pb6.into_alternate();
    let chan: ChannelBuilder<TIM4, 0, false, _> = ChannelBuilder::new(pwm_pin);
    let mut pwm = p.TIM4.pwm_hz(chan, 1.Hz(), &clocks);
    let max_duty = pwm.get_max_duty();
    pwm.enable(Channel::C1);
    pwm.set_duty(Channel::C1, max_duty);

    loop {
        cs.set_low();
        spi.transfer_in_place(&mut buffer).unwrap(); // Передаём и принимаем данные через SPI
        cs.set_high();

        let raw_value: u16 = ((buffer[0] as u16) << 8 | (buffer[1] as u16)) >> 2 & 0x0FFF; // Mask 12 bits

        // Реализация зависисимости скорости вращения от вентилятора
        if raw_value as f32 >= critical_threshold {
            // Если температура будет критической, сразу ставим на максимум
            pwm.set_duty(Channel::C1, max_duty);
            led.toggle(); // мигаем при критической температуре
        } else if raw_value as f32 > calculated_threshold {
            // Если температура больше заданной (50), идём динамически рассчитывать новый цикл работы пина
            // сенсор температуры выдает максимум 1.5 вольта, а сравниваем с 5 вольтами
            let duty_cycle =
                ((raw_value as f32 / adc_max_value * 5. / 1.5) * max_duty as f32) as u16;
            pwm.set_duty(Channel::C1, duty_cycle);
            led.set_high(); // также включаем светодиод
        } else {
            // иначе переходим в пассивный режим (нужно время на остановку)
            pwm.set_duty(Channel::C1, 0);
            led.set_low();
        }

        // Задержка перед следующим измерением (около 0.48 секунд)
        cortex_m::asm::delay(480000);
    }
}
