import mailchannelsPlugin from "@cloudflare/pages-plugin-mailchannels";

interface Env {
    EMAIL: string;
}

export const onRequest: PagesFunction<Env> = (context) =>
    mailchannelsPlugin({
        personalizations: [
            {
                to: [{name: "ACME Support", email: context.env.EMAIL}],
            },
        ],
        from: {name: "Enquiry", email: context.env.EMAIL},
        respondWith: () =>
            new Response(null, {
                status: 302,
                headers: {Location: "/thank-you"},
            }),
    })(context);

