const HOST = "http://" + Deno.env.get("DEPLOY_HOST");

const MODELS = [
    {
        "base_url": "https://api.openai.com/v1",
        "model_name": "gpt-4o-2024-11-20",
    },
];
const JUDGE = {
    "model": {
        "base_url": "https://api.openai.com/v1",
        "model_name": "gpt-4o-2024-11-20",
    },
    "prompt":
        'Please compare "{{expected_response}}" with "{{actual_response}}"',
};
const PARAMETER = {
    "temperature": 0.2,
    "seed": 99827122,
};
const AUTHORIZATION = {
    "Authorization": Deno.env.get("API_AUTHORIZATION")!,
};

Deno.test({
    name: "test input.csv",
    async fn() {
        const analyze_form = new FormData();
        analyze_form.append("data", new Blob([ await Deno.readFile("../input.csv") ], { type: "text/csv" }));
    
        analyze_form.append("models", new Blob([ JSON.stringify(MODELS) ], { type: "application/json" }));
        analyze_form.append("parameter", new Blob([ JSON.stringify(PARAMETER) ], { type: "application/json" }));
        analyze_form.append("judge", new Blob([ JSON.stringify(JUDGE) ], { type: "application/json" }));
        
        const analyze_request = await fetch(`${HOST}/analyze/`, {
            headers: {
                ...AUTHORIZATION,
            },
            method: "POST",
            body: analyze_form,
        });
    
        const { id: analysis_id } = await analyze_request.json();
        
        const analysis_result_url = await new Promise((res) => {
            (async() => {
                while (true) {
                    const analysis_status_request = await fetch(`${HOST}/analyze/${analysis_id}`, {
                        headers: {
                            ...AUTHORIZATION 
                        }
                    });
                    const { url } = await analysis_status_request.json();
                    
                    if (url) {
                        res(url);
                        break;
                    }
                    
                    await new Promise(res => setTimeout(res, 1000));
                }
            })();
        });
        
        console.log({ analysis_result_url });
        
        const post_analyze_form = new URLSearchParams();
        post_analyze_form.append("analysis_id", analysis_id);
        post_analyze_form.append("minimum_similarity", (0.5).toString());
        post_analyze_form.append("minimum_judge", (0.75).toString());
        
        const post_analyze_request = await fetch(`${HOST}/post_analyze/`, {
            headers: {
                ...AUTHORIZATION,
                "Content-Type": "application/x-www-form-urlencoded"
            },
            method: "POST",
            body: post_analyze_form,
        });
        const { success, message } = await post_analyze_request.json();
        
        if (!success)
            throw message;
    }
});
