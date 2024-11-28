#![deny(unsafe_code)]
#![no_main]
#![no_std]

// Import necessary crates
use cortex_m_rt::entry;
use fugit::Duration;
use panic_halt as _;
use stm32f4xx_hal::adc::config::AdcConfig;
use stm32f4xx_hal::prelude::*;
use stm32f4xx_hal::{adc, pac};

#[entry]
fn main() -> ! {
    // Take ownership of device peripherals
    let dp = pac::Peripherals::take().unwrap();

    // Configure the system clock
    let rcc = dp.RCC.constrain();
    let clocks = rcc.cfgr.sysclk(48.MHz()).freeze();

    // Configure GPIO and ADC
    let gpioa = dp.GPIOA.split();
    let mut adc_pin = gpioa.pa0.into_analog(); // PA0 connected to LM35

    // let adc_config = adc::config::AdcConfig;
    let mut adc = adc::Adc::adc1(dp.ADC1, true, AdcConfig::default());

    // Configure PWM (PB6 for TIM4)
    let gpiob = dp.GPIOB.split();
    let pwm_pin = gpiob.pb6.into_alternate(); // Use PB6 for TIM4 (Alternate function for TIM4_CH1)
    const DENOM: u32 = 1_000_000; // denominator (e.g., for microseconds)

    // Define the duration in ticks (e.g., 1 million ticks for 1 second)
    let pwm_freq: Duration<u32, 1, DENOM> = Duration::<u32, 1, DENOM>::from_ticks(1_000_000);

    // let mut pwm = dp.TIM4.pwm(pwm_pin, pwm_freq, &clocks);
    let (_, (ch1, ch2, ..)) = dp.TIM1.pwm_us(pwm_pin, pwm_freq, &clocks);
    let mut ch1 = ch1.with(gpioa.pa8);
    let mut _ch2 = ch2.with(gpioa.pa9);

    let max_duty = ch1.get_max_duty();
    ch1.set_duty(max_duty / 2);
    ch1.enable();

    pwm.enable();
    let max_duty = pwm.get_max_duty();

    // Main loop
    loop {
        // Read temperature from ADC
        let adc_value: u16 = adc.read(&mut adc_pin).unwrap();
        let temperature = (adc_value as f32) * 3.3 / 4095.0 * 100.0; // LM35 conversion

        // Map temperature to PWM duty cycle
        let duty = calculate_duty(temperature, max_duty);
        pwm.set_duty(duty);
    }
}

// Map temperature to PWM duty cycle
fn calculate_duty(temperature: f32, max_duty: u16) -> u16 {
    if temperature < 25.0 {
        0 // Fan off
    } else if temperature > 70.0 {
        max_duty // Full speed
    } else {
        ((temperature - 25.0) / 45.0 * max_duty as f32) as u16
    }
}
