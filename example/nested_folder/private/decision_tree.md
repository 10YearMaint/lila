---
output_filename: "decision_tree"
---

# Diagram

Here's a simple flowchart using *Mermaid*:

```mermaid
graph LR
    %% Main graph
    subgraph Local
        style Local fill:#f2f2f2,stroke:#bfbfbf,stroke-width:2px;
        A[IC/BC of xDE]:::dataNode
        B[Simulation Complete]:::dataNode
        C[Visualization in Lucy App]:::outputNode
    end

    subgraph Cloud
        style Cloud fill:#f2f2f2,stroke:#bfbfbf,stroke-width:2px;
        D[Simulation Results]:::dataNode
    end

    %% Data Flows
    A --> |Simulate Locally| B
    B --> |View Results| C

    A --> |Simulate Locally| B
    B --> |Send to Cloud| D

    D --> |Fetch from Cloud| B
    B --> |View Results| C

    %% Legend
    subgraph Legend["Legend"]
        direction TB
        style Legend fill:#f7f7f7,stroke:none;
        L1[<span style="color:#00b300">▶ Re-Simulate</span>]
        L2[<span style="color:#0066cc">⏩ Catch up with Cloud Database</span>]
        L3[<span style="color:#ff8000">☁️⬆️ Publisher Mode</span>]
    end

    %% Link Styles
    linkStyle 0 stroke:#00b300,stroke-width:3px;
    linkStyle 1 stroke:#00b300,stroke-width:3px;

    linkStyle 2 stroke:#ff8000,stroke-width:3px,stroke-dasharray: 5 5;
    linkStyle 3 stroke:#ff8000,stroke-width:3px,stroke-dasharray: 5 5;

    linkStyle 4 stroke:#0066cc,stroke-width:3px;
    linkStyle 5 stroke:#0066cc,stroke-width:3px;

    %% Node Styles
    classDef dataNode fill:#d9d1be,stroke:#333,stroke-width:2px;
    classDef outputNode fill:#81C7D4,stroke:#333,stroke-width:2px;
```
