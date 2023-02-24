import mailchannelsPlugin from "@cloudflare/pages-plugin-mailchannels";

export const onRequest = mailchannelsPlugin({
    personalizations: [
        {
            to: [{name: "ACME Support", email: process.env.EMAIL!}],
        },
    ],
    from: {name: "Enquiry", email: process.env.EMAIL!},
    respondWith: () =>
        new Response(null, {
            status: 302,
            headers: {Location: "/thank-you"},
        }),
});
