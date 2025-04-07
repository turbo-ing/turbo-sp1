# Turbo ZK V0

Turbo ZK V0 enable developers to build ZK indie games in less than 3 hours!

It's currently require a server (with Nvidia GPU) in this version.

## ZK and Server Part

### Writing rust code

A file for struct definition

```rust
use alloy_sol_types::sol;
use turbo_zk_v0::{TurboSerialize};
use serde::{Serialize, Deserialize};

sol! {
    #[derive(Serialize, Deserialize, Debug)]
    struct PlayerState {
        uint64 x;
        uint64 y;
        uint64 power;
        uint64 score;
    }

    #[derive(Serialize, Deserialize, Debug)]
    struct Shot {
        uint64 x;
        uint64 y;
        uint64 rad;
        uint64 timestamp;
    }

    #[derive(Serialize, Deserialize, Debug)]
    struct GamePublicState {
        PlayerState[] players;
    }

    #[derive(Serialize, Deserialize, Debug)]
    struct GamePrivateState {
        Shot[] shots;
    }

    struct MoveAction {
        uint8 dir;
    }

    struct AttackAction {
        uint64 rad;
    }
}

impl TurboInitState for GamePublicState {
    ...
}

impl TurboInitState for GamePrivateState {
    ...
}

enum GameAction {
    MoveAction(u8)
    AttackAction(u64)
}

impl TurboActionDeserialize for GameAction {
    ...
}
```

Another file for logic

```rust
pub fn process(public_state: &mut GamePublicState, private_state: &mut GamePrivateState, action: &GameAction, context: &TurboActionContext) {
    match action {
        GameAction::MoveAction(dir) => {
            ...
        },
        GameAction::AttackAction(rad) => {
            ...
        }
    }
}
```

And last file for entry point

### Server Player

TODO

### Running the 
