#![deny(unsafe_code)]
#![deny(warnings)]
#![no_main]
#![no_std]

// Остановка при панике
use panic_halt as _;

use cortex_m::iprintln;
use cortex_m_rt::entry;

use stm32f4xx_hal::{
    self as hal,
    adc::Adc,
    pac::{self, TIM4},
    prelude::*,
    timer::{Channel, ChannelBuilder},
};

#[entry]
fn main() -> ! {
    if let (Some(dp), Some(mut itm)) = (
        pac::Peripherals::take(),
        cortex_m::Peripherals::take().map(|p| p.ITM),
    ) {
        // Настраиваем тактирование системы на 84 МГц
        let rcc = dp.RCC.constrain();
        let clocks = rcc.cfgr.sysclk(84.MHz()).freeze();

        iprintln!(&mut itm.stim[0], "Программа запущена!");

        // Настраиваем GPIOB для управления транзистором через PB6 (ШИМ)
        let gpiob = dp.GPIOB.split();
        let pwm_pin = gpiob.pb6.into_alternate(); // Настраиваем PB6 как альтернативную функцию (ШИМ)

        // Создаем канал ШИМ для TIM4
        let chan: ChannelBuilder<TIM4, 0, false, _> = ChannelBuilder::new(pwm_pin);

        // Конфигурируем таймер TIM4 для работы с частотой ШИМ 20 кГц
        let mut pwm = dp.TIM4.pwm_hz(chan, 20.kHz(), &clocks);

        // Устанавливаем начальный цикл заполнения (duty cycle) 50%
        let max_duty = pwm.get_max_duty();
        pwm.set_duty(Channel::C1, max_duty / 2); // 50%
        pwm.enable(Channel::C1); // Включаем ШИМ

        // Настраиваем GPIOA для работы с АЦП (PA0 подключен к LM35)
        let gpioa = dp.GPIOA.split();
        let mut analog_pin = gpioa.pa0.into_analog();

        // Конфигурируем АЦП
        let mut adc = Adc::adc1(dp.ADC1, true, hal::adc::config::AdcConfig::default());

        loop {
            // Считываем значение с LM35 (PA0)
            let temperature_value: u16 = adc.read(&mut analog_pin).unwrap();

            // Преобразуем значение из АЦП в температуру (например, в градусах Цельсия)
            // LM35 дает 10 мВ/°C, АЦП STM32F4 имеет разрешение 12 бит (0–4095)
            // при референсном напряжении 3.3 В
            let temperature_celsius = (temperature_value as f32) * 3.3 / 4095.0 * 100.0;

            // Управляем скоростью мотора в зависимости от температуры
            if temperature_celsius > 60.0 {
                // Если температура выше 30 °C, увеличиваем мощность (75% duty cycle)
                pwm.set_duty(Channel::C1, (max_duty * 3) / 4);
            } else {
                // Если температура ниже или равна 30 °C, уменьшаем мощность (50% duty cycle)
                pwm.set_duty(Channel::C1, max_duty / 2);
            }

            cortex_m::asm::nop(); // Задержка (можно добавить таймер, если нужно)
        }
    }

    loop {
        cortex_m::asm::nop(); // Бесконечный цикл
    }
}
