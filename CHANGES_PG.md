# Changes in PG Rust sdk 1.0.2 (03-2026)
Bugfix 🐛:
- Add `hasOtherJoinedMember()` method that queries actual member state events from the store instead of relying on the potentially stale `joinedMembersCount` summary count

# Changes in PG Rust sdk 1.0.1 (03-2026)
Upstream merge ✨:

- Base update to matrix-sdk-ffi/20260305 (https://github.com/matrix-org/matrix-rust-sdk/tree/21e8138805247b1aa9a7230f6fc594579cb1b65c)
    - For element-x-Android v26.03.2 (https://github.com/element-hq/element-x-android/tree/v26.03.2)

# Changes in PG Rust sdk 1.0.0 (03-2026)
Upstream merge ✨:

- Base update to matrix-sdk-ffi/20260224 (https://github.com/matrix-org/matrix-rust-sdk/tree/6c75571c38202c071b08c2eeb390a2d15272f878)
    - For element-x-Android v26.03.0 (https://github.com/element-hq/element-x-android/tree/v26.03.0)

# Changes in PG Rust sdk 0.2.15
Improvements ✨:
- improves hidden users UX

# Changes in PG Rust sdk 0.2.14 (xx-xx-2025)
Improvements ✨:

- Invites appear always on top.

# Changes in PG Rust sdk 0.2.12 (xx-xx-2025)
Upstream merge ✨:

- Base update to matrix-sdk-ffi/20260113 (https://github.com/matrix-org/matrix-rust-sdk/tree/matrix-sdk-ffi/20260113) 
    - For element-x-ios v26.01.0 (https://github.com/element-hq/element-x-ios/releases/tag/release%2F26.01.0)

Bugfix 🐛:

- Fixes iOS background crash
    - related to email, function & departement info in room & room info
    - requires an update of Android to rely on roomInfo.heroes

# Changes in PG Rust sdk 0.2.11 (xx-xx-2025)
Upstream merge ✨:

- Base update to matrix-sdk-ffi/20251210 (https://github.com/matrix-org/matrix-rust-sdk/tree/matrix-sdk-ffi/20251210) 
    - For element-x-ios v25.12.1 (https://github.com/element-hq/element-x-ios/releases/tag/release%2F25.12.1)

Features ✨:
- Rework HasPhoneBookConsent to PhonebookConsentType

# Changes in PG Rust sdk 0.2.10 (xx-xx-2025)
Upstream merge ✨:

- Base update to matrix-sdk-ffi/20251202 (https://github.com/matrix-org/matrix-rust-sdk/tree/matrix-sdk-ffi/20251202) 
    - For element-x-android v25.12.0

# Changes in PG Rust sdk 0.2.9 (xx-xx-2025)
Features ✨:

- Add email to notification data

# Changes in PG Rust sdk 0.2.8 (xx-xx-2025)
Upstream merge ✨:

- Base update to 659ae572183c29f25bd1c27d444b4a92787b109e (https://github.com/matrix-org/matrix-rust-sdk/commit/659ae572183c29f25bd1c27d444b4a92787b109e) 
    - For element-x-android v25.09.2

# Changes in PG Rust sdk 0.2.7 (xx-xx-2025)
Features ✨:
- QR login reciprocation (login on web with mobile)

Improvements 🙌: 
- add network error to QR login flow

# Changes in PG Rust sdk 0.2.6 (xx-xx-2025)

Features ✨:
- Adds email property to room hero

# Changes in PG Rust sdk 0.2.5 (xx-xx-2025)

Upstream merge ✨:

Features ✨:
- Include isDirect (DM) info in element call widget url

Improvements 🙌:

Bugfix 🐛:

# Changes in PG Rust sdk 0.2.4 (xx-xx-2025)

Upstream merge ✨:

- Base update to 1eb8f6a (https://github.com/matrix-org/matrix-rust-sdk/commit/1eb8f6ac161fbf1c773a7ddb8243d7378489fdc8)

Features ✨:

Improvements 🙌:

Bugfix 🐛:


# Changes in PG Rust sdk 0.2.4 (xx-xx-2025)

Upstream merge ✨:

Features ✨:
- Get mxId of user by email using pg-pod-service

Improvements 🙌:

Bugfix 🐛:

- Remove pod-service url fetch causing slow load time, instead use matrix url override

# Changes in PG Rust sdk 0.2.3 (xx-xx-2025)

Upstream merge ✨:

- Base update to v0.13.0 (https://github.com/matrix-org/matrix-rust-sdk/tree/matrix-sdk-0.13.0)

Features ✨:

Improvements 🙌:

Bugfix 🐛:

# Changes in PG Rust sdk 0.2.2 (xx-xx-2025)

Upstream merge ✨:

Features ✨:

- Add new prop 'is_profile_initialized' to profile & ability to update the value of it.

Improvements 🙌:

Bugfix 🐛:

# Changes in PG Rust sdk 0.2.1 (xx-xx-2025)

Upstream merge ✨:

Features ✨:

- Expose profile data like department, function, email & phonebook consent.
- Make function & phonebook consent editable.

Improvements 🙌:

Bugfix 🐛:

# Changes in PG Rust sdk 0.2.0 (xx-xx-2025)

Upstream merge ✨:

Features ✨:

- Method for getting the pod_service url from the matrix client (from well-known)

Improvements 🙌:

Bugfix 🐛:

# Changes in PG Rust sdk 0.1.0 (xx-xx-2025)

Upstream merge ✨:

Features ✨:

- Extra property for room creation to set initial state of displayname for non-phonebook users
- Method to set initial state of displayname for non-phonebook users for existing rooms

Improvements 🙌:

Bugfix 🐛:

# Changes in PG Rust sdk 0.0.1 (xx-xx-2025)

Upstream merge ✨:

- Base update to v0.12.0 (https://github.com/matrix-org/matrix-rust-sdk/tree/matrix-sdk-0.12.0)

Features ✨:

- Add email prop to matrix user object.

Improvements 🙌:

Bugfix 🐛:
