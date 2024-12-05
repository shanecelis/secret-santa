# secret-santa: Finds a secret santa solution

This tool accepts an input file describing a set of people who wish to be
each other's secret santas. The hope is that by using this tool, the person
organizing it can do so blind to who each other's secret santas are.

Finding a secret santa solution imposes a number of constraints:

1. No one may be their own secret santa.

2. Each person is a secret santa for only _one_ other person.

3. Each person is a recipient for only _one_ other person.

The above rules feel like a given. The next set of rules are more
discretionary.

4. If X is a secret santa to Y, then Y is NOT is a secret santa to X.

It just seemed like little cycles like this wouldn't be fun. There can be
longer cycles though.

5. Optional but we do not permit members of the same household to be each
other's secret santa.

6. Optional the history of secret santas can be used to ensure that whomever
you got last year or the year before, you won't get them again. (You can't
go back indefinitely though otherwise there would be no solutions.)

# Input Sample

```
(
    people: [
        (
            name: "John",
            email: "john@email.com",
        ),
        (
            name: "Sean",
            email: "sean@email.com",
        ),
        (
            name: "Shane",
            email: "shane@email.com",
        ),
    ],
    whitelist: [
        (
            giver: "Sean",
            receiver: "Shane",
        ),
    ],
    blacklist: [
        (
            giver: "Sean",
            receiver: "Shane",
        ),
    ],
    blacklist_sets: [
        [
            "John",
            "Sean",
        ],
    ],
    history: [
        (
            year: 2024,
            exclude_pairs: true,
            pairs: [
                (
                    giver: "John",
                    receiver: "Shane",
                ),
                (
                    giver: "Sean",
                    receiver: "John",
                ),
                (
                    giver: "Shane",
                    receiver: "Sean",
                ),
            ],
        ),
    ],
)
```

# Code 

See code [here](/src/main.rs).
