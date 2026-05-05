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

use std::cmp::Ordering;

use matrix_sdk_base::RoomState;

use super::{RoomListItem, Sorter};

/// Create a new sorter that will sort two [`RoomListItem`] by their invite
/// state: rooms that are invites ([`RoomState::Invited`]) come first,
/// all other rooms come after.
///
/// This sorter ensures that pending invites are always displayed at the top
/// of the room list, making them immediately visible to users.
pub fn new_sorter() -> impl Sorter {
    move |left: &RoomListItem, right: &RoomListItem| -> Ordering {
        let left_is_invite = left.cached_state == RoomState::Invited;
        let right_is_invite = right.cached_state == RoomState::Invited;

        match (left_is_invite, right_is_invite) {
            // Both are invites, or neither is - treat as equal (let next sorter decide)
            (true, true) | (false, false) => Ordering::Equal,
            // Left is invite, right is not - left comes first
            (true, false) => Ordering::Less,
            // Right is invite, left is not - right comes first
            (false, true) => Ordering::Greater,
        }
    }
}

#[cfg(test)]
mod tests {
    use matrix_sdk::test_utils::logged_in_client_with_server;
    use matrix_sdk_base::RoomState;
    use matrix_sdk_test::async_test;
    use ruma::room_id;

    use super::{super::super::filters::new_rooms, *};

    fn cmp<F>(states: F, left: &RoomListItem, right: &RoomListItem) -> Ordering
    where
        F: Fn(&RoomListItem, &RoomListItem) -> (RoomState, RoomState),
    {
        let (left_state, right_state) = states(left, right);
        let left_is_invite = left_state == RoomState::Invited;
        let right_is_invite = right_state == RoomState::Invited;

        match (left_is_invite, right_is_invite) {
            (true, true) | (false, false) => Ordering::Equal,
            (true, false) => Ordering::Less,
            (false, true) => Ordering::Greater,
        }
    }

    #[async_test]
    async fn test_invite_vs_invite() {
        let (client, server) = logged_in_client_with_server().await;
        let [room_a, room_b] =
            new_rooms([room_id!("!a:b.c"), room_id!("!d:e.f")], &client, &server).await;

        // Both rooms are invites - they are equal (let next sorter decide).
        assert_eq!(
            cmp(|_, _| (RoomState::Invited, RoomState::Invited), &room_a, &room_b),
            Ordering::Equal
        );
    }

    #[async_test]
    async fn test_invite_vs_joined() {
        let (client, server) = logged_in_client_with_server().await;
        let [room_a, room_b] =
            new_rooms([room_id!("!a:b.c"), room_id!("!d:e.f")], &client, &server).await;

        // `room_a` is an invite, `room_b` is joined - invite comes first.
        assert_eq!(
            cmp(|_, _| (RoomState::Invited, RoomState::Joined), &room_a, &room_b),
            Ordering::Less
        );
    }

    #[async_test]
    async fn test_joined_vs_invite() {
        let (client, server) = logged_in_client_with_server().await;
        let [room_a, room_b] =
            new_rooms([room_id!("!a:b.c"), room_id!("!d:e.f")], &client, &server).await;

        // `room_a` is joined, `room_b` is an invite - invite comes first.
        assert_eq!(
            cmp(|_, _| (RoomState::Joined, RoomState::Invited), &room_a, &room_b),
            Ordering::Greater
        );
    }

    #[async_test]
    async fn test_non_invite_vs_non_invite() {
        let (client, server) = logged_in_client_with_server().await;
        let [room_a, room_b] =
            new_rooms([room_id!("!a:b.c"), room_id!("!d:e.f")], &client, &server).await;

        // Both rooms are joined - they are equal (let next sorter decide).
        assert_eq!(
            cmp(|_, _| (RoomState::Joined, RoomState::Joined), &room_a, &room_b),
            Ordering::Equal
        );

        // Both rooms are left - they are equal (let next sorter decide).
        assert_eq!(
            cmp(|_, _| (RoomState::Left, RoomState::Left), &room_a, &room_b),
            Ordering::Equal
        );

        // One joined, one left - neither is invite, so equal.
        assert_eq!(
            cmp(|_, _| (RoomState::Joined, RoomState::Left), &room_a, &room_b),
            Ordering::Equal
        );
    }

    #[async_test]
    async fn test_invite_vs_other_states() {
        let (client, server) = logged_in_client_with_server().await;
        let [room_a, room_b] =
            new_rooms([room_id!("!a:b.c"), room_id!("!d:e.f")], &client, &server).await;

        // Invite vs Left - invite comes first.
        assert_eq!(
            cmp(|_, _| (RoomState::Invited, RoomState::Left), &room_a, &room_b),
            Ordering::Less
        );

        // Invite vs Knocked - invite comes first.
        assert_eq!(
            cmp(|_, _| (RoomState::Invited, RoomState::Knocked), &room_a, &room_b),
            Ordering::Less
        );

        // Invite vs Banned - invite comes first.
        assert_eq!(
            cmp(|_, _| (RoomState::Invited, RoomState::Banned), &room_a, &room_b),
            Ordering::Less
        );
    }
}
