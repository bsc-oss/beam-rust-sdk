//
// Copyright 2026 Belgian Secure Communications (BSC)
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::future::IntoFuture;

use eyeball::SharedObservable;
use futures_core::Stream;
use matrix_sdk_base::crypto::types::qr_login::{QrCodeData, QrCodeIntent};
use oauth2::{EndUserVerificationUrl, VerificationUriComplete};
use tracing::trace;
use vodozemac::ecies::CheckCode;

use super::{
    messages::{LoginFailureReason, LoginProtocolType, QrAuthMessage},
    secure_channel::EstablishedSecureChannel,
    QRCodeLoginError, SecureChannelError,
};
use crate::Client;

async fn send_unexpected_message_error(
    channel: &mut EstablishedSecureChannel,
) -> Result<(), SecureChannelError> {
    channel
        .send_json(QrAuthMessage::LoginFailure {
            reason: LoginFailureReason::UnexpectedMessageReceived,
            homeserver: None,
        })
        .await
}

/// Type telling us about the progress of the QR code reciprocation.
#[derive(Clone, Debug, Default)]
pub enum ReciprocateProgress {
    /// We're just starting up, this is the default and initial state.
    #[default]
    Starting,
    /// We have established the secure channel, but we need to let the other
    /// side know about the [`CheckCode`] so they can verify that the secure
    /// channel is indeed secure.
    EstablishingSecureChannel {
        /// The check code we need to, out of band, compare with the other device.
        check_code: CheckCode,
    },
    /// We are waiting for the user to confirm the login at the verification URL.
    ConfirmingLogin {
        /// The verification URL the user should open to authorize the login.
        verification_uri: EndUserVerificationUrl,
        /// The verification URL with an embedded user code, if present.
        verification_uri_complete: Option<VerificationUriComplete>,
    },
    /// The reciprocation process has completed.
    Done,
}

/// Named future for the `OAuth::reciprocate_with_qr_code()` method.
#[derive(Debug)]
pub struct ReciprocateWithQrCode<'a> {
    client: &'a Client,
    qr_code_data: &'a QrCodeData,
    state: SharedObservable<ReciprocateProgress>,
}

impl ReciprocateWithQrCode<'_> {
    /// Subscribe to the progress of QR code reciprocation.
    pub fn subscribe_to_progress(&self) -> impl Stream<Item = ReciprocateProgress> + use<> {
        self.state.subscribe()
    }
}

impl<'a> IntoFuture for ReciprocateWithQrCode<'a> {
    type Output = Result<(), QRCodeLoginError>;
    type IntoFuture = std::pin::Pin<
        Box<dyn Future<Output = Self::Output> + Send + 'a>,
    >;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(async move {
            // Establish the secure channel; we scanned a Login (0x03) QR, so we expect the other side to be Login.
            let mut channel = self.establish_secure_channel().await?;

            trace!("Established the secure channel (reciprocation).");

            // Show the check code to the user for verification.
            let check_code = channel.check_code().to_owned();
            self.state.set(ReciprocateProgress::EstablishingSecureChannel { check_code });

            // Advertise supported protocols and homeserver.
            trace!("Advertising supported protocols to the new device.");
            let homeserver = self.client.homeserver();
            channel
                .send_json(QrAuthMessage::LoginProtocols {
                    protocols: vec![LoginProtocolType::DeviceAuthorizationGrant],
                    homeserver,
                })
                .await?;

            // Expect the protocol selection from the new device.
            let first_message = channel.receive_json().await?;
            match first_message {
                QrAuthMessage::LoginProtocol { device_authorization_grant, protocol, device_id: _ } => {
                    if protocol != LoginProtocolType::DeviceAuthorizationGrant {
                        channel
                            .send_json(QrAuthMessage::LoginFailure {
                                reason: LoginFailureReason::UnsupportedProtocol,
                                homeserver: None,
                            })
                            .await?;
                        return Err(QRCodeLoginError::LoginFailure {
                            reason: LoginFailureReason::UnsupportedProtocol,
                            homeserver: None,
                        });
                    }

                    // Update progress to allow the UI to open the verification URL.
                    self.state.set(ReciprocateProgress::ConfirmingLogin {
                        verification_uri: device_authorization_grant.verification_uri.clone(),
                        verification_uri_complete: device_authorization_grant
                            .verification_uri_complete
                            .clone(),
                    });

                    // Accept the chosen protocol.
                    trace!("Accepting login protocol.");
                    channel.send_json(QrAuthMessage::LoginProtocolAccepted).await?;

                    // Wait for the new device to complete the OAuth device flow.
                    // Avoid background polling while the system browser is in the foreground.
                    // Give the host app control to resume once the browser is closed.
                    trace!("Waiting for device grant (app resumes after browser)");
                    self.client.oauth().wait_for_device_grant().await;
                    // After resume, proceed to wait for the outcome.
                    let msg = channel.receive_json().await?;

                    match msg {
                        QrAuthMessage::LoginSuccess => (),
                        QrAuthMessage::LoginDeclined => {
                            return Err(QRCodeLoginError::LoginFailure {
                                reason: LoginFailureReason::UserCancelled,
                                homeserver: None,
                            })
                        }
                        QrAuthMessage::LoginFailure { reason, homeserver } => {
                            return Err(QRCodeLoginError::LoginFailure { reason, homeserver })
                        }
                        message => {
                            send_unexpected_message_error(&mut channel).await?;
                            return Err(QRCodeLoginError::UnexpectedMessage {
                                expected: "m.login.success",
                                received: message,
                            });
                        }
                    }
                }
                message => {
                    send_unexpected_message_error(&mut channel).await?;
                    return Err(QRCodeLoginError::UnexpectedMessage {
                        expected: "m.login.protocol",
                        received: message,
                    });
                }
            }

            // Export and send the secrets bundle so the new device becomes fully E2EE-capable.
            trace!("Exporting secrets bundle for the new device.");
            let bundle = self
                .client
                .encryption()
                .export_secrets_bundle()
                .await
                .map_err(QRCodeLoginError::SecretExport)?;

            trace!("Sending secrets bundle.");
            channel.send_json(QrAuthMessage::LoginSecrets(bundle)).await?;

            trace!("successfully logged in other device and enabled E2EE.");

            // All done.
            self.state.set(ReciprocateProgress::Done);
            Ok(())
        })
    }
}

impl<'a> ReciprocateWithQrCode<'a> {
    pub(crate) fn new(client: &'a Client, qr_code_data: &'a QrCodeData) -> Self {
        Self { client, qr_code_data, state: Default::default() }
    }

    async fn establish_secure_channel(&self) -> Result<EstablishedSecureChannel, SecureChannelError> {
        let http_client = self.client.inner.http_client.inner.clone();
        let channel = EstablishedSecureChannel::from_qr_code(
            http_client,
            self.qr_code_data,
            QrCodeIntent::Reciprocate,
        )
        .await?;
        Ok(channel)
    }
}

#[cfg(all(test, not(target_family = "wasm")))]
mod test {
    use std::time::Duration;

    use assert_matches2::assert_let;
    use futures_util::StreamExt;
    use matrix_sdk_common::executor::spawn;
    use matrix_sdk_test::async_test;
    use serde_json::json;

    use super::*;
    use crate::{
        authentication::oauth::qrcode::secure_channel::{test::MockedRendezvousServer, SecureChannel},
        http_client::HttpClient,
        test_utils::mocks::MatrixMockServer,
    };

    const TEST_RENDEZVOUS_SESSION: &str = "abcdEFG12345";
    const TEST_DEVICE_ID: &str = "wjLpTLRqbqBzLs63aYaEv2Boi6cFEbbM/sSRQ2oAKk4";
    const TEST_VERIFICATION_URI: &str = "https://id.example.org/device/abcde";
    const TEST_VERIFICATION_URI_COMPLETE: &str = "https://id.example.org/device/abcde?code=ABCDE";

    enum NewDeviceBehaviour {
        HappyPath,
        UnsupportedProtocol,
        UnexpectedFirst,
        UnexpectedInsteadOfSuccess,
        Declined,
        ExplicitFailure,
    }

    async fn simulate_new_device(
        alice: SecureChannel,
        check_code_rx: tokio::sync::oneshot::Receiver<CheckCode>,
        behaviour: NewDeviceBehaviour,
    ) {
        // Connect and confirm secure channel using the out-of-band check code
        let alice = alice.connect().await.expect("Alice should connect secure channel");
        let check_code = check_code_rx.await.expect("Should receive check code from existing device");
        let mut alice = alice.confirm(check_code.to_digit()).expect("Alice should confirm channel");

        // Expect protocols advertisement from existing device
        let msg: QrAuthMessage = alice.receive_json().await.expect("Alice should receive protocols");
        assert_let!(QrAuthMessage::LoginProtocols { .. } = msg);

        match behaviour {
            NewDeviceBehaviour::UnexpectedFirst => {
                // Send success prematurely (unexpected in phase 1)
                let unexpected = json!({ "type": "m.login.success" });
                alice.send_json(&unexpected).await.unwrap();
                return;
            }
            NewDeviceBehaviour::UnsupportedProtocol => {
                // Send a protocol with an unsupported type
                let login_protocol = json!({
                    "type": "m.login.protocol",
                    "protocol": "com.example.unsupported",
                    "device_authorization_grant": {
                        "verification_uri": TEST_VERIFICATION_URI,
                        "verification_uri_complete": TEST_VERIFICATION_URI_COMPLETE
                    },
                    "device_id": TEST_DEVICE_ID
                });
                alice.send_json(&login_protocol).await.unwrap();
                // Existing device will send a failure and terminate; swallow one message if sent
                let _ = alice.receive_json::<serde_json::Value>().await.ok();
                return;
            }
            _ => {
                // Send a valid device_authorization_grant protocol selection
                let login_protocol = json!({
                    "type": "m.login.protocol",
                    "protocol": "device_authorization_grant",
                    "device_authorization_grant": {
                        "verification_uri": TEST_VERIFICATION_URI,
                        "verification_uri_complete": TEST_VERIFICATION_URI_COMPLETE
                    },
                    "device_id": TEST_DEVICE_ID
                });
                alice.send_json(&login_protocol).await.unwrap();

                // Expect protocol accepted
                let msg: QrAuthMessage = alice.receive_json().await.unwrap();
                assert_let!(QrAuthMessage::LoginProtocolAccepted = msg);

                match behaviour {
                    NewDeviceBehaviour::Declined => {
                        let declined = json!({ "type": "m.login.declined" });
                        alice.send_json(&declined).await.unwrap();
                    }
                    NewDeviceBehaviour::ExplicitFailure => {
                        let failure = json!({
                            "type": "m.login.failure",
                            "reason": "device_not_found",
                        });
                        alice.send_json(&failure).await.unwrap();
                    }
                    NewDeviceBehaviour::UnexpectedInsteadOfSuccess => {
                        // Send an unexpected message in phase 2
                        let unexpected = json!({
                            "type": "m.login.protocol",
                            "protocol": "device_authorization_grant",
                            "device_authorization_grant": {
                                "verification_uri": TEST_VERIFICATION_URI,
                                "verification_uri_complete": TEST_VERIFICATION_URI_COMPLETE
                            },
                            "device_id": TEST_DEVICE_ID
                        });
                        alice.send_json(&unexpected).await.unwrap();
                    }
                    NewDeviceBehaviour::HappyPath => {
                        // Send success, then expect secrets
                        let success = json!({ "type": "m.login.success" });
                        alice.send_json(&success).await.unwrap();

                        let msg: QrAuthMessage = alice.receive_json().await.unwrap();
                        assert_let!(QrAuthMessage::LoginSecrets(_bundle) = msg);
                    }
                    _ => {}
                }
            }
        }
    }

    async fn setup_existing_device_and_qr(
        server: &MatrixMockServer,
    ) -> (Client, QrCodeData, SecureChannel, MockedRendezvousServer) {
        let rendezvous_server = MockedRendezvousServer::new(server.server(), TEST_RENDEZVOUS_SESSION, Duration::MAX).await;

        let client = HttpClient::new(reqwest::Client::new(), Default::default());
        let alice = SecureChannel::login(client, &rendezvous_server.homeserver_url)
            .await
            .expect("Should create secure channel");

        // Alice is the new device, so she should create a Login QR code
        let qr_code_data = alice.qr_code_data().clone();

        // Mock the versions endpoint to avoid auto-discovery failure
        server.mock_versions().ok().mount().await;

        // Existing device client - use the crypto-aware client builder for proper token handling
        let existing = server
            .client_builder_for_crypto_end_to_end(
                &ruma::owned_user_id!("@example:localhost"),
                &ruma::owned_device_id!("DEVICEID"),
            )
            .build()
            .await;

        // Set up crypto endpoints and bootstrap cross-signing
        server.mock_crypto_endpoints_preset().await;
        existing.encryption().bootstrap_cross_signing(None).await.expect("bootstrap x-signing");

        let qr_code = qr_code_data;
        // Return components needed for testing
        (existing, qr_code, alice, rendezvous_server)
    }

    /// Helper to set up progress subscription and coordinate check code transfer
    async fn setup_reciprocate_test(
        existing: Client,
        qr_code: QrCodeData,
        alice: SecureChannel,
        behaviour: NewDeviceBehaviour,
    ) -> Result<(), QRCodeLoginError> {
        let oauth = existing.oauth();
        let reciprocate = oauth.reciprocate_with_qr_code(&qr_code);
        let mut updates = reciprocate.subscribe_to_progress();
        let (tx, rx) = tokio::sync::oneshot::channel();

        let oauth_for_updates = oauth.clone();
        let _updates_task = spawn(async move {
            let mut sender = Some(tx);
            while let Some(update) = updates.next().await {
                match update {
                    ReciprocateProgress::EstablishingSecureChannel { check_code } => {
                        sender
                            .take()
                            .expect("The establishing secure channel update should be received only once")
                            .send(check_code)
                            .expect("Should send check code to new device");
                    }
                    ReciprocateProgress::ConfirmingLogin { .. } => {
                        // In tests, immediately resume to simulate closing the browser.
                        oauth_for_updates.resume_qr_reciprocation().await;
                    }
                    ReciprocateProgress::Done => break,
                    _ => {}
                }
            }
        });

        let alice_task = spawn(async move {
            simulate_new_device(alice, rx, behaviour).await;
        });

        let result = reciprocate.await;
        alice_task.await.unwrap();
        result
    }

    #[async_test]
    async fn test_qr_reciprocate_happy_path() {
        let server = MatrixMockServer::new().await;
        let (existing, qr_code, alice, _rendezvous_server) = setup_existing_device_and_qr(&server).await;

        // Set up progress subscription to forward the check code
        let oauth = existing.oauth();
        let reciprocate = oauth.reciprocate_with_qr_code(&qr_code);
        let mut updates = reciprocate.subscribe_to_progress();
        let (tx, rx) = tokio::sync::oneshot::channel();

        let oauth_for_updates = oauth.clone();
        let updates_task = spawn(async move {
            let mut sender = Some(tx);

            while let Some(update) = updates.next().await {
                match update {
                    ReciprocateProgress::EstablishingSecureChannel { check_code } => {
                        sender
                            .take()
                            .expect("The establishing secure channel update should be received only once")
                            .send(check_code)
                            .expect("Should send check code to new device");
                    }
                    ReciprocateProgress::ConfirmingLogin { .. } => {
                        // Simulate returning from the browser by signaling resume.
                        oauth_for_updates.resume_qr_reciprocation().await;
                    }
                    ReciprocateProgress::Done => break,
                    _ => {}
                }
            }
        });

        let alice_task = spawn(async move {
            simulate_new_device(alice, rx, NewDeviceBehaviour::HappyPath).await;
        });

        let result = reciprocate.await;
        alice_task.await.unwrap();
        updates_task.await.unwrap();

        assert!(result.is_ok(), "Existing device should complete reciprocation successfully");
    }

    #[async_test]
    async fn test_qr_reciprocate_unsupported_protocol() {
        let server = MatrixMockServer::new().await;
        let (existing, qr_code, alice, _rendezvous_server) = setup_existing_device_and_qr(&server).await;

        let result = setup_reciprocate_test(existing, qr_code, alice, NewDeviceBehaviour::UnsupportedProtocol).await;

        assert_let!(Err(QRCodeLoginError::LoginFailure { reason, .. }) = result);
        assert_eq!(reason, LoginFailureReason::UnsupportedProtocol);
    }

    #[async_test]
    async fn test_qr_reciprocate_unexpected_first_message() {
        let server = MatrixMockServer::new().await;
        let (existing, qr_code, alice, _rendezvous_server) = setup_existing_device_and_qr(&server).await;

        let result = setup_reciprocate_test(existing, qr_code, alice, NewDeviceBehaviour::UnexpectedFirst).await;

        assert_let!(Err(QRCodeLoginError::UnexpectedMessage { expected, .. }) = result);
        assert_eq!(expected, "m.login.protocol");
    }

    #[async_test]
    async fn test_qr_reciprocate_unexpected_instead_of_success() {
        let server = MatrixMockServer::new().await;
        let (existing, qr_code, alice, _rendezvous_server) = setup_existing_device_and_qr(&server).await;

        let result = setup_reciprocate_test(existing, qr_code, alice, NewDeviceBehaviour::UnexpectedInsteadOfSuccess).await;

        assert_let!(Err(QRCodeLoginError::UnexpectedMessage { expected, .. }) = result);
        assert_eq!(expected, "m.login.success");
    }

    #[async_test]
    async fn test_qr_reciprocate_declined() {
        let server = MatrixMockServer::new().await;
        let (existing, qr_code, alice, _rendezvous_server) = setup_existing_device_and_qr(&server).await;

        let result = setup_reciprocate_test(existing, qr_code, alice, NewDeviceBehaviour::Declined).await;

        assert_let!(Err(QRCodeLoginError::LoginFailure { reason, .. }) = result);
        assert_eq!(reason, LoginFailureReason::UserCancelled);
    }

    #[async_test]
    async fn test_qr_reciprocate_explicit_failure() {
        let server = MatrixMockServer::new().await;
        let (existing, qr_code, alice, _rendezvous_server) = setup_existing_device_and_qr(&server).await;

        let result = setup_reciprocate_test(existing, qr_code, alice, NewDeviceBehaviour::ExplicitFailure).await;

        assert_let!(Err(QRCodeLoginError::LoginFailure { reason, .. }) = result);
        assert_eq!(reason, LoginFailureReason::DeviceNotFound);
    }
}
