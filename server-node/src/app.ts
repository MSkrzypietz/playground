import express from 'express';
import bodyParser from 'body-parser';
import cors from 'cors';
import {PrismaClient} from '@prisma/client'

const prisma = new PrismaClient()

const app = express();

app.use(bodyParser.json());
app.use(cors());

app.get('/', (req, res) => {
    res.send({yo: 'hi'});
});

app.get('/test', async (req, res) => {
    const repos = await prisma.repository.findMany({take: 10, include: {owner: true, languages: true}});
    res.json({repos});
});


app.listen(3000, () => {
    console.log('Node.js app listening on port 3000!');
});
