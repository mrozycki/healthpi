use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use chrono::Utc;
use futures::stream;
use healthpi_bt::{BleCharacteristicEvent, MockBleCharacteristic, MockBleDevice, MockBleSession};
use healthpi_loader::{
    devices::{device::MockFactory, soehnle::Shape200},
    Loader,
};
use mockall::predicate::eq;
use uuid::Uuid;

const WEIGHT_CUSTOM_CHARACTERISTIC: Uuid = Uuid::from_u128(0x352e3001_28e9_40b8_a361_6db4cca4147c);
const CMD_CHARACTERISTIC: Uuid = Uuid::from_u128(0x352e3002_28e9_40b8_a361_6db4cca4147c);
const CUSTOM_SERVICE_UUID: Uuid = Uuid::from_u128(0x352e3000_28e9_40b8_a361_6db4cca4147c);

#[tokio::test]
async fn shape_200_returns_no_records() {
    let running = Arc::new(AtomicBool::new(true));
    let running_clone = running.clone();

    let mut ble_session = MockBleSession::new();
    ble_session.expect_start_discovery().returning(|| Ok(()));
    ble_session.expect_get_devices().returning(move || {
        running_clone.store(false, Ordering::Relaxed);
        let mut ble_device = MockBleDevice::new();
        ble_device.expect_connect().returning(|| Ok(()));
        ble_device
            .expect_get_characteristic()
            .with(eq(CUSTOM_SERVICE_UUID), eq(WEIGHT_CUSTOM_CHARACTERISTIC))
            .returning(|_, _| {
                let mut ble_characteristic = MockBleCharacteristic::new();
                ble_characteristic.expect_subscribe().returning(|| {
                    Ok(Box::pin(stream::iter([BleCharacteristicEvent {
                        value: vec![12, 1, 1, 29, 0, 0, 187, 0, 0, 1],
                    }])))
                });
                Ok(Box::new(ble_characteristic))
            });
        ble_device
            .expect_get_characteristic()
            .with(eq(CUSTOM_SERVICE_UUID), eq(CMD_CHARACTERISTIC))
            .returning(|_, _| {
                let mut ble_characteristic = MockBleCharacteristic::new();
                ble_characteristic
                    .expect_write_with_response()
                    .with(eq(vec![0x0c, 0x01]))
                    .returning(|_| Ok(()));
                ble_characteristic
                    .expect_write_with_response()
                    .with(eq(vec![0x09, 0x01]))
                    .returning(|_| Ok(()));
                Ok(Box::new(ble_characteristic))
            });
        ble_device.expect_disconnect().returning(|| Ok(()));
        Ok(vec![Box::new(ble_device)])
    });
    ble_session.expect_stop_discovery().returning(|| Ok(()));

    let mut factory = MockFactory::new();
    factory
        .expect_make_device()
        .returning(|ble_device| Some(Box::new(Shape200::new(ble_device))));
    factory.expect_mark_processed().returning(|_| Utc::now());

    let mut measurement_repository = healthpi_client::MockClient::new();
    measurement_repository
        .expect_post_records()
        .with(eq(vec![]))
        .returning(|_| Ok(()));

    let loader = Arc::new(Loader::new(
        Box::new(ble_session),
        Box::new(factory),
        Box::new(measurement_repository),
        running,
    ));

    loader.run().await.unwrap();
}
