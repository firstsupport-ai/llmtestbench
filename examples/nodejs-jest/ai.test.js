const fs = require("fs");

const HOST = "http://152.42.210.216";

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
    "Authorization": "9421111BCDB7FC53C11F0B0885640E68"
};

test("test input.csv", async () => {
    const analyze_form = new (require("form-data"))();
    analyze_form.append("data", fs.readFileSync("../input.csv"), {
        contentType: "text/csv",
    });

    analyze_form.append("models", JSON.stringify(MODELS), {
        contentType: "application/json",
    });
    analyze_form.append("parameter", JSON.stringify(PARAMETER), {
        contentType: "application/json",
    });
    analyze_form.append("judge", JSON.stringify(JUDGE), {
        contentType: "application/json",
    });
    
    const analyze_request = await fetch(`${HOST}/analyze/`, {
        headers: {
            ...AUTHORIZATION,
            ...analyze_form.getHeaders()
        },
        method: "POST",
        body: analyze_form.getBuffer(),
    });

    const { id: analysis_id } = await analyze_request.json();
    
    const analysis_result_url = await new Promise(async (res) => {
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
    });
    
    console.log({ analysis_result_url });
    
    const post_analyze_form = new URLSearchParams();
    post_analyze_form.append("analysis_id", analysis_id);
    post_analyze_form.append("minimum_similarity", 0.5);
    post_analyze_form.append("minimum_judge", 0.75);
    
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
}, 60000);
