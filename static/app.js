const e = React.createElement;
const DateTime = luxon.DateTime;
const CONNECTION_LOST_TIMEOUT = 10;

const Sidebar = ({ currentModel, previousModels, popularModels }) => {
    const showPreviousModels = previousModels && previousModels.length > 0;
    const showPopularModels = previousModels && popularModels.length > 0;

    return e(
        "div",
        { id: "sidebar" },
        e(Logo),
        e(Card, { label: "Current Model" }, e(CurrentModel, { model: currentModel })),
        showPreviousModels && e(Card, { label: "Previous Models" }, e(ModelList, { models: previousModels })),
        showPopularModels && e(Card, { label: "Most Popular Models" }, e(ModelList, { models: popularModels }))
    );
};

const Logo = () => {
    return e("img", { src: "opti.png", alt: "Logo", id: "logo" });
};

const Card = ({ label, children }) => {
    return e(
        "div",
        { className: "card" },
        e("div", { className: "label" }, label),
        children
    );
};

const CurrentModel = ({model}) => {
    return e(
        "div",
        { id: "current-model" },
        e("span", { className: "model-name" }, model.name),
        e("span", { className: "model-detail" }, `${model.detail}`)
    );
};

const ModelList = ({ models }) => {
    return e(
        "div",
        { className: "model-list" },
        models.map((model) => {
            return e(
                "div",
                { className: "model-row" },
                e("span", { className: "model-name" }, model.name),
                e("span", { className: "model-detail" }, model.detail)
            );
        })
    );
};


// Message Types:
// Selection { model: String, actions: Option<SimpleControllerInput>, }
// Statistics { model: String, counts: u64, }
// e.g. { "Selection": { "model": "Kickoff", "actions": { ... } } }
// e.g. { "Statistics": { "model": "Kickoff", "counts": 100 } }

// SimpleControllerInput { throttle: f32, steer: f32, pitch: f32, yaw: f32, roll: f32, jump: bool, boost: bool, handbrake: bool, }
class API {
    constructor() {
        const protocol = window.location.protocol === "https:" ? "wss:" : "ws:";
        const host = window.location.host;
        this.uri = `${protocol}//${host}/ws`;
        this.socket = null;
        this.onCurrentModelEvent = () => {};
        this.onStatisticsEvent = () => {};
    }

    connect() {
        if (this.socket) {
            return;
        }
        this.socket = new WebSocket(this.uri);
        this.socket.onmessage = (event) => this.onMessage(event);
        this.socket.onclose = () => this.onClose();
        this.socket.onerror = (err) => this.onError(err);
    }

    disconnect() {
        if (this.socket) {
            this.socket.close();
        }
    }

    onMessage(event) {
        const data = JSON.parse(event.data);
        if (data.Selection) {
            this.onCurrentModelEvent(data.Selection);
        }
        if (data.Statistics) {
            this.onStatisticsEvent(data.Statistics);
        }
    }

    onClose() {
        console.log("Socket is closed. Reconnect will be attempted in 1 second.");
        setTimeout(() => this.connect(), 1000);
        this.socket = null;
    }

    onError(err) {
        console.error("Socket encountered error: ", err.message ?? "Unknown connection error");
        this.socket.close();
    }

    registerSelectionListener(listener) {
        this.onCurrentModelEvent = listener;
    }

    registerStatisticsListener(listener) {
        this.onStatisticsEvent = listener;
    }

}

class App extends React.Component {
    constructor(props) {
        super(props);
        this.state = {
            currentModel: null,
            currentModelView: null,
            modelHistory: [],
            modelUsage: {},
        };
        this.api = new API();
        this.ticker = null;
        this.lastEventReceived = null;
    }

    componentDidMount() {
        this.api.registerSelectionListener((event) => this.onCurrentModelEvent(event));
        this.api.registerStatisticsListener((event) => this.onStatisticsEvent(event));
        this.api.connect();
        this.ticker = setInterval(() => this.update(), 100);
    }

    componentWillUnmount() {
        this.api.disconnect();
        clearInterval(this.ticker);
    }

    currentModelDuration() {
        return DateTime.now().diff(this.state.currentModel.startTime).as("seconds").toFixed(3);
    }

    onCurrentModelEvent(event) {
        const modelName = event.model;

        if (this.state.currentModel && this.state.currentModel.name === modelName) {
            return;
        }

        if (this.state.currentModel) {
            this.state.modelHistory = [{
                name: this.state.currentModel.name,
                duration: `${this.currentModelDuration()}s`,
            }, ...this.state.modelHistory.slice(0, 2)]
        }

        this.state.currentModel = { name: modelName, startTime: DateTime.now() };

        // Predicts model usage before the statistics event is received.
        const model = this.state.modelUsage[modelName];
        if (model) {
            this.state.modelUsage[modelName] = model + 1;
        } else {
            this.state.modelUsage[modelName] = 1;
        }

        this.update(true);
    }

    onStatisticsEvent(event) {
        const modelName = event.model;
        const counts = event.counts;
        this.state.modelUsage[modelName] = counts;
        this.update(true);
    }

    popularModelsView() {
        const total = Object.values(this.state.modelUsage).reduce((a, b) => a + b, 0);
        return Object.entries(this.state.modelUsage)
            .map(([name, count]) => ({ name, count }))
            .sort((a, b) => b.count - a.count)
            .map((model) => ({ name: model.name, detail: `${(model.count / total * 100).toFixed(2)}%` }));
    }

    invalidateOnNoEvents() {
        if (!this.lastEventReceived) {
            return;
        }
        const now = DateTime.now();
        const elapsed = now.diff(this.lastEventReceived).as("seconds");
        if (elapsed > CONNECTION_LOST_TIMEOUT) {
            this.setState({
                currentModel: null,
            });
        }
    }

    update(receivedEvent) {
        if (receivedEvent) {
            this.lastEventReceived = DateTime.now();
        } else {
            this.invalidateOnNoEvents();
        }

        if (!this.state.currentModel) {
            this.setState({
                currentModelView: { name: "None", detail: "connecting..." },
            })
            return;
        }

        const currentModelView = {
            name: this.state.currentModel.name,
            detail: `${this.currentModelDuration()}s`,
        };

        const previousModelsView = this.state.modelHistory.map((model) => ({
            name: model.name,
            detail: model.duration,
        }));

        const popularModelsView = this.popularModelsView();

        this.setState({
            currentModelView,
            previousModelsView,
            popularModelsView,
        });
    }

    render() {
        if (!this.state.currentModelView) {
            return;
        }

        return e(Sidebar, {
            currentModel: this.state.currentModelView,
            previousModels: this.state.previousModelsView,
            popularModels: this.state.popularModelsView,
        });
    }
}

function createAppElement() {
    return e(App);
}
